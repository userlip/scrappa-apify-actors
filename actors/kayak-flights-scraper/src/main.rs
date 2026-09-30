use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::PathBuf,
    process::ExitCode,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};
use serde_json::{Map, Value, json};
use tokio::time::sleep;

const DEFAULT_SCRAPPA_URL: &str = "https://scrappa.co/api";
const DEFAULT_APIFY_URL: &str = "https://api.apify.com";
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_SCRAPPA_RETRIES: usize = 4;
const MAX_APIFY_RETRIES: usize = 8;
const MAX_BATCH_SIZE: usize = 100;
const USER_AGENT: &str = "ScrappaApifyActor/1.0 (+https://scrappa.co)";

const SPEC_JSON: &str = include_str!("../spec.json");
#[cfg(test)]
const FIXTURE_JSON: &str = include_str!("../fixtures/response.json");

#[derive(Debug)]
struct ActorSpec {
    title: String,
    endpoint: String,
    endpoint_by_input: HashMap<String, HashMap<String, String>>,
    endpoint_by_input_default: Option<String>,
    mode: String,
    batch: BatchSpec,
    parameters: Vec<ParameterSpec>,
    result_pointer: Option<String>,
    fallback_result_pointers: Vec<String>,
    pagination: Option<PaginationSpec>,
    enrichment: EnrichmentSpec,
    max_results: MaxResultsSpec,
    default_max_pages: usize,
}

#[derive(Debug)]
struct BatchSpec {
    field: String,
    value_field: String,
    api_param: String,
    path_param: Option<String>,
}

#[derive(Debug)]
struct ParameterSpec {
    input: String,
    api_param: String,
    location: String,
    required: bool,
    required_for_endpoints: Vec<String>,
}

#[derive(Debug)]
struct PaginationSpec {
    kind: String,
    param: String,
    start: Value,
    step: i64,
    next_pointer: Option<String>,
    next_pointers: Vec<String>,
    has_more_pointer: Option<String>,
    end_pointer: Option<String>,
    current_page_pointer: Option<String>,
    total_pages_pointer: Option<String>,
    max_pages: usize,
}

#[derive(Debug)]
struct EnrichmentSpec {
    field: String,
}

#[derive(Debug)]
struct MaxResultsSpec {
    input: String,
    default: usize,
    hard_limit: usize,
}

#[derive(Debug)]
struct RequestEntry {
    endpoint: String,
    value: Value,
    params: Map<String, Value>,
    path_params: Map<String, Value>,
}

struct Storage {
    http: Client,
    local_root: Option<PathBuf>,
    apify_base: Url,
    token: Option<String>,
    key_value_store_id: Option<String>,
    dataset_id: Option<String>,
    run_id: Option<String>,
    input_key: String,
    remaining_items: usize,
    local_next_item: usize,
}

impl Storage {
    fn from_env() -> Result<Self> {
        let local_mode = env::var("APIFY_LOCAL_MODE")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
        let apify_base = Url::parse(
            &env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| DEFAULT_APIFY_URL.to_owned()),
        )
        .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?;
        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .retry(reqwest::retry::never())
            .user_agent(USER_AGENT)
            .build()
            .context("Could not create the Apify HTTP client")?;

        if local_mode {
            let local_root = env::var_os("APIFY_LOCAL_STORAGE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("storage"));
            let dataset_dir = local_root.join("datasets/default");
            fs::create_dir_all(&dataset_dir)
                .context("Could not create the local Apify dataset directory")?;
            let local_next_item = fs::read_dir(&dataset_dir)
                .context("Could not read the local Apify dataset directory")?
                .filter_map(std::result::Result::ok)
                .filter(|entry| {
                    entry.path().extension().and_then(|ext| ext.to_str()) == Some("json")
                })
                .count();
            return Ok(Self {
                http,
                local_root: Some(local_root),
                apify_base,
                token: None,
                key_value_store_id: None,
                dataset_id: None,
                run_id: None,
                input_key: "INPUT".to_owned(),
                remaining_items: usize::MAX,
                local_next_item,
            });
        }

        Ok(Self {
            http,
            local_root: None,
            apify_base,
            token: Some(required_env("APIFY_TOKEN")?),
            key_value_store_id: Some(required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?),
            dataset_id: Some(required_env("ACTOR_DEFAULT_DATASET_ID")?),
            run_id: Some(required_env("ACTOR_RUN_ID")?),
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "INPUT".to_owned()),
            remaining_items: usize::MAX,
            local_next_item: 0,
        })
    }

    async fn get_input(&self) -> Result<Value> {
        if let Some(root) = &self.local_root {
            let path = env::var_os("APIFY_LOCAL_INPUT_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("key_value_stores/default/INPUT.json"));
            let input = fs::read_to_string(&path)
                .with_context(|| format!("Could not read local input at {}", path.display()))?;
            return serde_json::from_str(&input).context("Local Actor input was not valid JSON");
        }

        let url = self.api_url(&[
            "v2",
            "key-value-stores",
            self.key_value_store_id.as_deref().unwrap_or_default(),
            "records",
            &self.input_key,
        ])?;
        let response = self
            .send_apify_with_retry("input retrieval", || {
                self.authorized(Method::GET, url.clone())
            })
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            bail!("Apify INPUT was not found in the default key-value store");
        }
        response_json(response, "read Apify INPUT").await
    }

    async fn initialize_budget(&mut self, requested: usize) -> Result<()> {
        if self.local_root.is_some() {
            self.remaining_items = requested;
            return Ok(());
        }

        let run_id = self.run_id.as_deref().unwrap_or_default();
        let url = self.api_url(&["v2", "actor-runs", run_id])?;
        let response = self
            .send_apify_with_retry("run pricing request", || {
                self.authorized(Method::GET, url.clone())
            })
            .await?;
        let run = response_json(response, "read Actor run pricing").await?;
        let mut allowed = apify_dataset_budget(&run, requested)?;

        if let Some(max_paid_items) = configured_paid_item_limit()? {
            allowed = allowed.min(remaining_paid_dataset_items(&run, max_paid_items)?);
        }

        self.remaining_items = allowed;
        Ok(())
    }

    async fn push_items(&mut self, items: &[Value]) -> Result<usize> {
        let count = items.len().min(self.remaining_items);
        if count == 0 {
            return Ok(0);
        }

        if let Some(root) = &self.local_root {
            let dataset_dir = root.join("datasets/default");
            fs::create_dir_all(&dataset_dir)
                .context("Could not create the local Apify dataset directory")?;
            for item in &items[..count] {
                self.local_next_item += 1;
                let path = dataset_dir.join(format!("{:09}.json", self.local_next_item));
                fs::write(&path, serde_json::to_vec(item)?).with_context(|| {
                    format!(
                        "Could not write local dataset item {}",
                        self.local_next_item
                    )
                })?;
            }
        } else {
            let dataset_id = self.dataset_id.as_deref().unwrap_or_default();
            let url = self.api_url(&["v2", "datasets", dataset_id, "items"])?;
            let response = self
                .authorized(Method::POST, url)
                .json(&items[..count])
                .send()
                .await
                .context("Could not write items to the Apify dataset")?;
            require_success(response, "write dataset items").await?;
        }

        self.remaining_items -= count;
        Ok(count)
    }

    async fn set_status_message(&self, message: &str) -> Result<()> {
        if self.local_root.is_some() {
            println!("Status: {message}");
            return Ok(());
        }

        let run_id = self.run_id.as_deref().unwrap_or_default();
        let url = self.api_url(&["v2", "actor-runs", run_id])?;
        let response = self
            .send_apify_with_retry("status message update", || {
                self.authorized(Method::PATCH, url.clone())
                    .json(&json!({"statusMessage": message}))
            })
            .await?;
        require_success(response, "update Actor status message").await?;
        Ok(())
    }

    fn authorized(&self, method: Method, url: Url) -> RequestBuilder {
        let request = self
            .http
            .request(method, url)
            .header(reqwest::header::ACCEPT, "application/json");
        match &self.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn api_url(&self, segments: &[&str]) -> Result<Url> {
        append_segments(&self.apify_base, segments)
    }

    async fn send_apify_with_retry<F>(&self, operation: &str, mut request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        for attempt in 0..=MAX_APIFY_RETRIES {
            match request().send().await {
                Ok(response) if should_retry(response.status()) && attempt < MAX_APIFY_RETRIES => {
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Apify {operation} returned {}; retrying after {}ms",
                        response.status(),
                        delay.as_millis()
                    );
                    sleep(delay).await;
                }
                Ok(response) => return Ok(response),
                Err(_) if attempt < MAX_APIFY_RETRIES => {
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Apify {operation} failed; retrying after {}ms",
                        delay.as_millis()
                    );
                    sleep(delay).await;
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("Apify {operation} failed after retries"));
                }
            }
        }
        bail!("Apify {operation} request exhausted its retry loop")
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn parse_spec() -> Result<ActorSpec> {
    let value: Value =
        serde_json::from_str(SPEC_JSON).context("Compiled Actor spec is invalid JSON")?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("Compiled Actor spec must be an object"))?;
    let string = |name: &str| -> Result<String> {
        object
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("Compiled Actor spec is missing {name}"))
    };
    let number = |name: &str, fallback: usize| {
        object
            .get(name)
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(fallback)
    };
    let batch = object
        .get("batch")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Compiled Actor spec is missing batch configuration"))?;
    let batch_string = |name: &str| -> Result<String> {
        batch
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("Compiled Actor batch spec is missing {name}"))
    };
    let parameters = object
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|parameter| {
            let parameter_object = parameter
                .as_object()
                .ok_or_else(|| anyhow!("Compiled Actor parameter must be an object"))?;
            Ok(ParameterSpec {
                input: parameter_object
                    .get("input")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Compiled Actor parameter is missing input"))?
                    .to_owned(),
                api_param: parameter_object
                    .get("apiParam")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Compiled Actor parameter is missing apiParam"))?
                    .to_owned(),
                location: parameter_object
                    .get("location")
                    .and_then(Value::as_str)
                    .unwrap_or("query")
                    .to_owned(),
                required: parameter_object
                    .get("required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                required_for_endpoints: parameter_object
                    .get("requiredForEndpoints")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let endpoint_by_input = object
        .get("endpointByInput")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(input, choices)| {
            let choices = choices
                .as_object()
                .ok_or_else(|| anyhow!("Endpoint choices for {input} must be an object"))?
                .iter()
                .map(|(value, endpoint)| {
                    let endpoint = endpoint
                        .as_str()
                        .ok_or_else(|| anyhow!("Endpoint choice must be a string"))?;
                    Ok((value.clone(), endpoint.to_owned()))
                })
                .collect::<Result<HashMap<_, _>>>()?;
            Ok((input.clone(), choices))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    let pagination = object
        .get("pagination")
        .filter(|value| !value.is_null())
        .map(|value| {
            let pagination = value
                .as_object()
                .ok_or_else(|| anyhow!("Compiled Actor pagination must be an object"))?;
            let optional_string = |name: &str| {
                pagination
                    .get(name)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            };
            Ok::<PaginationSpec, anyhow::Error>(PaginationSpec {
                kind: pagination
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("page")
                    .to_owned(),
                param: pagination
                    .get("param")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Compiled Actor pagination is missing param"))?
                    .to_owned(),
                start: pagination.get("start").cloned().unwrap_or(Value::from(1)),
                step: pagination.get("step").and_then(Value::as_i64).unwrap_or(1),
                next_pointer: optional_string("nextPointer"),
                next_pointers: pagination
                    .get("nextPointers")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                has_more_pointer: optional_string("hasMorePointer"),
                end_pointer: optional_string("endPointer"),
                current_page_pointer: optional_string("currentPagePointer"),
                total_pages_pointer: optional_string("totalPagesPointer"),
                max_pages: pagination
                    .get("maxPages")
                    .and_then(Value::as_u64)
                    .unwrap_or(1) as usize,
            })
        })
        .transpose()?;
    let enrichment = object
        .get("enrichment")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Compiled Actor spec is missing enrichment"))?;
    let max_results = object
        .get("maxResults")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Compiled Actor spec is missing maxResults"))?;

    Ok(ActorSpec {
        title: string("title")?,
        endpoint: string("endpoint")?,
        endpoint_by_input,
        endpoint_by_input_default: object
            .get("endpointByInputDefault")
            .and_then(Value::as_str)
            .map(str::to_owned),
        mode: string("mode")?,
        batch: BatchSpec {
            field: batch_string("field")?,
            value_field: batch_string("valueField")?,
            api_param: batch_string("apiParam")?,
            path_param: batch
                .get("pathParam")
                .and_then(Value::as_str)
                .map(str::to_owned),
        },
        parameters,
        result_pointer: object
            .get("resultPointer")
            .and_then(Value::as_str)
            .map(str::to_owned),
        fallback_result_pointers: object
            .get("fallbackResultPointers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        pagination,
        enrichment: EnrichmentSpec {
            field: enrichment
                .get("field")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Compiled Actor spec is missing enrichment field"))?
                .to_owned(),
        },
        max_results: MaxResultsSpec {
            input: max_results
                .get("input")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Compiled Actor spec is missing maxResults input"))?
                .to_owned(),
            default: max_results
                .get("default")
                .and_then(Value::as_u64)
                .unwrap_or(100) as usize,
            hard_limit: max_results
                .get("hardLimit")
                .and_then(Value::as_u64)
                .unwrap_or(1000) as usize,
        },
        default_max_pages: number("defaultMaxPages", 1),
    })
}

fn env_or_default(name: &str, fallback: &str) -> String {
    env::var(name).unwrap_or_else(|_| fallback.to_owned())
}

fn configured_paid_item_limit() -> Result<Option<usize>> {
    match env::var("ACTOR_MAX_PAID_DATASET_ITEMS") {
        Ok(value) => {
            let limit = value
                .parse::<usize>()
                .context("ACTOR_MAX_PAID_DATASET_ITEMS must be a non-negative integer")?;
            Ok(Some(limit))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error).context("ACTOR_MAX_PAID_DATASET_ITEMS was not valid Unicode"),
    }
}

fn remaining_paid_dataset_items(run: &Value, max_paid_items: usize) -> Result<usize> {
    let data = run.get("data").unwrap_or(run);
    let charged_items = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .and_then(|counts| counts.get(DATASET_ITEM_EVENT))
        .map(|count| {
            count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {DATASET_ITEM_EVENT}"))
        })
        .transpose()?
        .unwrap_or(0) as usize;

    Ok(max_paid_items.saturating_sub(charged_items))
}

fn apify_dataset_budget(run: &Value, requested: usize) -> Result<usize> {
    let data = run.get("data").unwrap_or(run);
    let pricing = data.get("pricingInfo").unwrap_or(&Value::Null);
    if pricing.get("pricingModel").and_then(Value::as_str) != Some("PAY_PER_EVENT") {
        return Ok(requested);
    }

    let events = pricing
        .pointer("/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run pricing did not include event prices"))?;
    let item_price = events
        .get(DATASET_ITEM_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| {
            anyhow!("Apify run pricing did not include the default dataset item event")
        })?;
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run pricing returned an invalid dataset item price");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let environment_max_charge = match env::var("ACTOR_MAX_TOTAL_CHARGE_USD") {
        Ok(value) => Some(
            value
                .parse::<f64>()
                .context("ACTOR_MAX_TOTAL_CHARGE_USD must be a number")?,
        ),
        Err(env::VarError::NotPresent) => None,
        Err(error) => {
            return Err(error).context("ACTOR_MAX_TOTAL_CHARGE_USD was not valid Unicode");
        }
    };
    let max_charge = environment_max_charge.or_else(|| {
        data.pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
    });
    if max_charge.is_some_and(|value| !value.is_finite() || value < 0.0) {
        bail!("Apify run returned an invalid maximum charge");
    }
    let Some(max_charge) = max_charge.filter(|value| *value > 0.0) else {
        return Ok(requested);
    };
    if !max_charge.is_finite() {
        bail!("Apify run returned an invalid maximum charge");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut spent = 0.0;
    for (event_name, count) in &counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Apify run provided an invalid event count"))?;
        if count == 0 {
            continue;
        }
        let price = events
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide a price for a charged event"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Apify run provided invalid event pricing");
        }
        spent += price * count as f64;
    }

    let remaining_charge = (max_charge - spent).max(0.0);
    Ok(((remaining_charge / item_price).floor() as usize).min(requested))
}

fn parse_requests(input: &Value, spec: &ActorSpec) -> Result<Vec<RequestEntry>> {
    let object = input
        .as_object()
        .ok_or_else(|| anyhow!("Actor input must be a JSON object"))?;
    let entries = object
        .get(&spec.batch.field)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Input field {} must be an array", spec.batch.field))?;
    if entries.is_empty() {
        bail!("Provide at least one entry in {}", spec.batch.field);
    }
    if entries.len() > MAX_BATCH_SIZE {
        bail!(
            "{} accepts at most {MAX_BATCH_SIZE} batch entries per run",
            spec.title
        );
    }

    let endpoint = selected_endpoint(input, spec)?;
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let value = batch_value(entry, &spec.batch.value_field).ok_or_else(|| {
                anyhow!(
                    "{} entry {} is missing {}",
                    spec.batch.field,
                    index + 1,
                    spec.batch.value_field
                )
            })?;
            if value.is_null() || value.as_str().is_some_and(str::is_empty) {
                bail!(
                    "{} entry {} has an empty {}",
                    spec.batch.field,
                    index + 1,
                    spec.batch.value_field
                );
            }

            let mut params = Map::new();
            let mut path_params = Map::new();
            for parameter in &spec.parameters {
                let source = entry
                    .as_object()
                    .and_then(|entry_object| entry_object.get(&parameter.input))
                    .or_else(|| object.get(&parameter.input));
                let source = if parameter.input == spec.batch.value_field {
                    Some(&value)
                } else {
                    source
                };
                let Some(source) = source.filter(|value| !value.is_null()) else {
                    continue;
                };
                if parameter.location == "path" {
                    path_params.insert(parameter.api_param.clone(), source.clone());
                } else {
                    params.insert(parameter.api_param.clone(), source.clone());
                }
            }
            if let Some(path_param) = &spec.batch.path_param {
                path_params.insert(path_param.clone(), value.clone());
            } else if !spec.batch.api_param.is_empty() {
                params.insert(spec.batch.api_param.clone(), value.clone());
            }

            validate_required_params(&params, &path_params, &endpoint, spec, index)?;
            Ok(RequestEntry {
                endpoint: endpoint.clone(),
                value,
                params,
                path_params,
            })
        })
        .collect()
}

fn batch_value(entry: &Value, value_field: &str) -> Option<Value> {
    match entry {
        Value::Object(object) => object.get(value_field).cloned(),
        Value::String(_) | Value::Number(_) => Some(entry.clone()),
        _ => None,
    }
}

fn validate_required_params(
    params: &Map<String, Value>,
    path_params: &Map<String, Value>,
    endpoint: &str,
    spec: &ActorSpec,
    index: usize,
) -> Result<()> {
    for parameter in &spec.parameters {
        let required = parameter.required
            || parameter
                .required_for_endpoints
                .iter()
                .any(|fragment| endpoint.contains(fragment));
        if !required {
            continue;
        }
        if parameter.location == "path" {
            if path_params
                .get(&parameter.api_param)
                .is_none_or(Value::is_null)
            {
                bail!(
                    "Batch entry {} requires path parameter {}",
                    index + 1,
                    parameter.api_param
                );
            }
        } else if params.get(&parameter.api_param).is_none_or(Value::is_null) {
            // Batch primary values are inserted after parameter mappings.
            if parameter.api_param == spec.batch.api_param && spec.batch.path_param.is_none() {
                continue;
            }
            bail!(
                "Input is missing required parameter {} for batch entry {}",
                parameter.input,
                index + 1
            );
        }
    }
    Ok(())
}

fn selected_endpoint(input: &Value, spec: &ActorSpec) -> Result<String> {
    let Some(by_input) = spec.endpoint_by_input.iter().next() else {
        return Ok(spec.endpoint.clone());
    };
    let (input_field, choices) = by_input;
    let selected = input
        .get(input_field)
        .and_then(Value::as_str)
        .or(spec.endpoint_by_input_default.as_deref())
        .ok_or_else(|| anyhow!("Input field {input_field} is required"))?;
    choices
        .get(selected)
        .cloned()
        .ok_or_else(|| anyhow!("Unsupported {input_field} value {selected}"))
}

fn max_results(input: &Value, spec: &ActorSpec) -> Result<usize> {
    let configured = match input.get(&spec.max_results.input) {
        Some(value) => value
            .as_u64()
            .map(|number| number as usize)
            .ok_or_else(|| anyhow!("{} must be a positive integer", spec.max_results.input))?,
        None => spec.max_results.default,
    };
    if configured == 0 {
        bail!("{} must be at least 1", spec.max_results.input);
    }
    Ok(configured.min(spec.max_results.hard_limit))
}

fn max_pages(input: &Value, spec: &ActorSpec) -> Result<usize> {
    let Some(pagination) = &spec.pagination else {
        return Ok(1);
    };
    let configured = match input.get("maxPages") {
        Some(value) => value
            .as_u64()
            .map(|number| number as usize)
            .ok_or_else(|| anyhow!("maxPages must be a positive integer"))?,
        None => spec.default_max_pages,
    };
    if configured == 0 {
        bail!("maxPages must be at least 1");
    }
    let hard_limit = if pagination.max_pages == 0 {
        spec.default_max_pages
    } else {
        pagination.max_pages
    };
    Ok(configured.min(hard_limit))
}

fn response_items(response: &Value, spec: &ActorSpec) -> Vec<Value> {
    if spec.mode == "single" {
        return (!response.is_null())
            .then(|| response.clone())
            .into_iter()
            .collect();
    }

    let pointers = spec
        .result_pointer
        .iter()
        .chain(spec.fallback_result_pointers.iter());
    for pointer in pointers {
        let value = if pointer.is_empty() {
            Some(response)
        } else {
            response.pointer(pointer)
        };
        if let Some(items) = value.and_then(Value::as_array) {
            return items.clone();
        }
    }
    Vec::new()
}

fn enrich_item(item: Value, input_value: &Value, field: &str, timestamp: &str) -> Value {
    let mut object = match item {
        Value::Object(object) => object,
        other => {
            let mut object = Map::new();
            object.insert("value".to_owned(), other);
            object
        }
    };
    object.insert(field.to_owned(), input_value.clone());
    object.insert("scraped_at".to_owned(), Value::String(timestamp.to_owned()));
    Value::Object(object)
}

fn timestamp_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_timestamp(seconds)
}

fn format_timestamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let day_seconds = seconds % 86_400;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn has_next_page(response: &Value, page_had_rows: bool, pagination: &PaginationSpec) -> bool {
    if let Some(end_pointer) = &pagination.end_pointer {
        if response.pointer(end_pointer).and_then(Value::as_bool) == Some(true) {
            return false;
        }
    }
    if let (Some(current), Some(total)) = (
        pagination
            .current_page_pointer
            .as_ref()
            .and_then(|pointer| response.pointer(pointer))
            .and_then(Value::as_u64),
        pagination
            .total_pages_pointer
            .as_ref()
            .and_then(|pointer| response.pointer(pointer))
            .and_then(Value::as_u64),
    ) {
        return current < total;
    }
    if let Some(pointer) = &pagination.has_more_pointer {
        if let Some(has_more) = response.pointer(pointer).and_then(Value::as_bool) {
            return has_more;
        }
    }
    for pointer in pagination
        .next_pointer
        .iter()
        .chain(pagination.next_pointers.iter())
    {
        if response
            .pointer(pointer)
            .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
        {
            return true;
        }
    }
    page_had_rows
}

fn next_page_value(
    response: &Value,
    params: &Map<String, Value>,
    pagination: &PaginationSpec,
) -> Value {
    for pointer in pagination
        .next_pointer
        .iter()
        .chain(pagination.next_pointers.iter())
    {
        if let Some(value) = response.pointer(pointer).filter(|value| !value.is_null()) {
            return value.clone();
        }
    }
    let current = params.get(&pagination.param).unwrap_or(&pagination.start);
    match current {
        Value::Number(number) => {
            let next = number
                .as_i64()
                .unwrap_or(0)
                .saturating_add(pagination.step.max(1));
            json!(next)
        }
        Value::String(value) if pagination.kind == "cursor" => json!(value),
        _ => pagination.start.clone(),
    }
}

fn endpoint_url(base: &Url, endpoint: &str, path_params: &Map<String, Value>) -> Result<Url> {
    let mut url = base.clone();
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("Scrappa base URL cannot contain path segments"))?;
        segments.pop_if_empty();
        for segment in endpoint.trim_start_matches('/').split('/') {
            if let Some(name) = segment
                .strip_prefix('{')
                .and_then(|value| value.strip_suffix('}'))
            {
                let value = path_params
                    .get(name)
                    .ok_or_else(|| anyhow!("Missing path parameter {name}"))?;
                segments.push(&value_to_string(value));
            } else {
                segments.push(segment);
            }
        }
    }
    Ok(url)
}

fn append_segments(base: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base.clone();
    let mut path = url
        .path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?;
    path.pop_if_empty().extend(segments.iter().copied());
    drop(path);
    Ok(url)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => String::new(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
        _ => value.to_string(),
    }
}

async fn request_scrappa(
    http: &Client,
    base: &Url,
    entry: &RequestEntry,
    api_key: &str,
    spec: &ActorSpec,
    row_limit: usize,
    page_limit: usize,
) -> Result<Value, EntryFailure> {
    let mut params = entry.params.clone();
    let mut seen_pages = HashSet::new();
    let mut pending_items = Vec::new();
    let mut page = 0;
    loop {
        let mut request_params = params.clone();
        if let Some(pagination) = &spec.pagination {
            if page == 0 && !request_params.contains_key(&pagination.param) {
                request_params.insert(pagination.param.clone(), pagination.start.clone());
            }
        }
        if let Some(pagination) = &spec.pagination {
            if let Some(value) = request_params.get(&pagination.param) {
                let key = value_to_string(value);
                if !seen_pages.insert(key) {
                    break;
                }
            }
        }

        let url = endpoint_url(base, &entry.endpoint, &entry.path_params)
            .map_err(|_| EntryFailure::Request)?;
        let mut url = url;
        {
            let mut query = url.query_pairs_mut();
            for (name, value) in &request_params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                query.append_pair(name, &value_to_string(value));
            }
        }

        let response = send_scrappa_with_retry(http, url, api_key).await?;
        let body = response.text().await.map_err(|_| EntryFailure::Request)?;
        let response: Value = serde_json::from_str(&body).map_err(|_| EntryFailure::InvalidJson)?;
        let rows = response_items(&response, spec);
        let page_had_rows = !rows.is_empty();
        pending_items.extend(
            rows.into_iter()
                .take(row_limit.saturating_sub(pending_items.len())),
        );

        let Some(pagination) = &spec.pagination else {
            break;
        };
        let page_limit = page_limit.min(pagination.max_pages.max(1));
        if pending_items.len() >= row_limit
            || page + 1 >= page_limit
            || !has_next_page(&response, page_had_rows, pagination)
        {
            break;
        }
        params.insert(
            pagination.param.clone(),
            next_page_value(&response, &request_params, pagination),
        );
        page += 1;
    }

    let timestamp = timestamp_now();
    let items = pending_items
        .into_iter()
        .map(|item| enrich_item(item, &entry.value, &spec.enrichment.field, &timestamp))
        .collect();
    Ok(Value::Array(items))
}

#[derive(Debug)]
enum EntryFailure {
    Http(StatusCode),
    Request,
    InvalidJson,
}

async fn send_scrappa_with_retry(
    http: &Client,
    url: Url,
    api_key: &str,
) -> Result<Response, EntryFailure> {
    for attempt in 0..=MAX_SCRAPPA_RETRIES {
        let result = http
            .get(url.clone())
            .header("X-API-KEY", api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await;
        match result {
            Ok(response) if should_retry(response.status()) && attempt < MAX_SCRAPPA_RETRIES => {
                let delay = retry_delay(attempt);
                eprintln!(
                    "Scrappa returned {}; retrying after {}ms",
                    response.status(),
                    delay.as_millis()
                );
                sleep(delay).await;
            }
            Ok(response) if !response.status().is_success() => {
                let status = response.status();
                return Err(EntryFailure::Http(status));
            }
            Ok(response) => return Ok(response),
            Err(_) if attempt < MAX_SCRAPPA_RETRIES => {
                let delay = retry_delay(attempt);
                eprintln!(
                    "Scrappa request failed; retrying after {}ms",
                    delay.as_millis()
                );
                sleep(delay).await;
            }
            Err(_) => return Err(EntryFailure::Request),
        }
    }
    Err(EntryFailure::Request)
}

fn should_retry(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(500_u64.saturating_mul(2_u64.saturating_pow(attempt as u32)))
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let response = require_success(response, operation).await?;
    response
        .json()
        .await
        .with_context(|| format!("{operation} did not contain valid JSON"))
}

async fn require_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    bail!(
        "Apify returned HTTP {} while trying to {operation}",
        response.status()
    );
}

async fn execute() -> Result<()> {
    let spec = parse_spec()?;
    let mut storage = Storage::from_env()?;
    let input = storage.get_input().await?;
    let requests = parse_requests(&input, &spec)?;
    let requested_max = max_results(&input, &spec)?;
    let page_limit = max_pages(&input, &spec)?;
    let api_key = required_env("SCRAPPA_API_KEY")?;
    let scrappa_base = Url::parse(&env_or_default("SCRAPPA_API_BASE_URL", DEFAULT_SCRAPPA_URL))
        .context("SCRAPPA_API_BASE_URL must be a valid URL")?;
    let scrappa_http = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .retry(reqwest::retry::never())
        .user_agent(USER_AGENT)
        .build()
        .context("Could not create the Scrappa HTTP client")?;
    storage.initialize_budget(requested_max).await?;

    println!(
        "{}: processing {} batch entr{}",
        spec.title,
        requests.len(),
        if requests.len() == 1 { "y" } else { "ies" }
    );
    let mut successful_entries = 0;
    let mut failed_entries = 0;
    let mut saved_items = 0;
    let mut empty_entries = 0;

    for (index, entry) in requests.iter().enumerate() {
        if saved_items >= requested_max || storage.remaining_items == 0 {
            break;
        }
        let remaining = requested_max
            .saturating_sub(saved_items)
            .min(storage.remaining_items);
        match request_scrappa(
            &scrappa_http,
            &scrappa_base,
            entry,
            &api_key,
            &spec,
            remaining,
            page_limit,
        )
        .await
        {
            Ok(value) => {
                let mut items = value.as_array().cloned().unwrap_or_default();
                items.truncate(remaining);
                successful_entries += 1;
                if items.is_empty() {
                    empty_entries += 1;
                    println!("Batch entry {} succeeded with no results", index + 1);
                    continue;
                }
                let pushed = storage.push_items(&items).await?;
                saved_items += pushed;
                println!("Batch entry {} saved {} dataset item(s)", index + 1, pushed);
            }
            Err(failure) => {
                failed_entries += 1;
                match failure {
                    EntryFailure::Http(status) => eprintln!(
                        "Batch entry {} failed with Scrappa HTTP {}",
                        index + 1,
                        status
                    ),
                    EntryFailure::Request => eprintln!(
                        "Batch entry {} failed after Scrappa request retries",
                        index + 1
                    ),
                    EntryFailure::InvalidJson => {
                        eprintln!("Batch entry {} returned invalid Scrappa JSON", index + 1)
                    }
                }
            }
        }
    }

    if successful_entries == 0 && failed_entries > 0 {
        bail!(
            "All {} batch entries failed. Review the per-entry Scrappa errors above and correct the input or retry later.",
            requests.len()
        );
    }

    let message = if saved_items == 0 && successful_entries == 0 {
        "No requests were sent because the result budget is exhausted.".to_owned()
    } else if saved_items == 0 {
        format!(
            "Completed {} successful request(s). No results were found.",
            successful_entries
        )
    } else {
        format!(
            "Completed {} successful request(s); saved {} result(s).",
            successful_entries, saved_items
        )
    };
    storage.set_status_message(&message).await?;
    if failed_entries > 0 {
        println!(
            "{} request entr{} failed; successful entries were retained",
            failed_entries,
            if failed_entries == 1 { "y" } else { "ies" }
        );
    }
    if empty_entries > 0 {
        println!(
            "{} successful entr{} returned no results",
            empty_entries,
            if empty_entries == 1 { "y" } else { "ies" }
        );
    }
    println!("{}", message);
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = execute().await {
        eprintln!("Actor failed: {error:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_fixture_has_rows_at_the_configured_result_location() {
        let spec = parse_spec().unwrap();
        let fixture: Value = serde_json::from_str(FIXTURE_JSON).unwrap();
        let rows = response_items(&fixture, &spec);
        assert!(
            !rows.is_empty(),
            "synthetic fixture must have at least one result"
        );
    }

    #[test]
    fn enriches_an_item_without_dropping_source_fields() {
        let item = enrich_item(
            json!({"title": "Synthetic result"}),
            &json!("example"),
            "input_query",
            "2026-01-01T00:00:00Z",
        );
        assert_eq!(item["title"], "Synthetic result");
        assert_eq!(item["input_query"], "example");
        assert_eq!(item["scraped_at"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn formats_unix_epoch_as_an_iso_timestamp() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn inserts_a_path_parameter_as_one_encoded_segment() {
        let base = Url::parse("https://scrappa.co/api").unwrap();
        let params = Map::from_iter([("id".to_owned(), json!("123 456"))]);
        let url = endpoint_url(&base, "/immobilienscout24/property/{id}", &params).unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/immobilienscout24/property/123%20456"
        );
    }

    #[test]
    fn pagination_uses_observed_next_page_token() {
        let pagination = PaginationSpec {
            kind: "cursor".to_owned(),
            param: "cursor".to_owned(),
            start: json!("0"),
            step: 1,
            next_pointer: Some("/data/cursor".to_owned()),
            next_pointers: Vec::new(),
            has_more_pointer: Some("/data/hasMore".to_owned()),
            end_pointer: None,
            current_page_pointer: None,
            total_pages_pointer: None,
            max_pages: 3,
        };
        let response = json!({"data": {"cursor": "200", "hasMore": true}});
        assert!(has_next_page(&response, true, &pagination));
        let params = Map::from_iter([("cursor".to_owned(), json!("0"))]);
        assert_eq!(next_page_value(&response, &params, &pagination), "200");
    }

    #[test]
    fn stops_cursor_pagination_when_the_response_says_there_are_no_more_rows() {
        let pagination = PaginationSpec {
            kind: "cursor".to_owned(),
            param: "cursor".to_owned(),
            start: json!("0"),
            step: 1,
            next_pointer: Some("/data/cursor".to_owned()),
            next_pointers: Vec::new(),
            has_more_pointer: Some("/data/hasMore".to_owned()),
            end_pointer: None,
            current_page_pointer: None,
            total_pages_pointer: None,
            max_pages: 2,
        };
        let response = json!({"data": {"cursor": "200", "hasMore": false}});
        assert!(!has_next_page(&response, true, &pagination));
    }

    #[test]
    fn computes_pay_per_event_budget_from_existing_charges() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.10},
                        "other-event": {"eventPriceUsd": 0.05}
                    }}
                },
                "chargedEventCounts": {"apify-default-dataset-item": 1, "other-event": 1},
                "options": {"maxTotalChargeUsd": 0.35}
            }
        });
        assert_eq!(apify_dataset_budget(&run, 10).unwrap(), 1);
    }

    #[test]
    fn applies_paid_item_limit_to_run_charges_including_zero() {
        let run = json!({
            "data": {
                "chargedEventCounts": {
                    "apify-default-dataset-item": 3
                }
            }
        });

        assert_eq!(remaining_paid_dataset_items(&run, 10).unwrap(), 7);
        assert_eq!(remaining_paid_dataset_items(&run, 2).unwrap(), 0);
        assert_eq!(remaining_paid_dataset_items(&run, 0).unwrap(), 0);
    }

    #[test]
    fn validates_the_prefilled_batch_input() {
        let spec = parse_spec().unwrap();
        let input: Value = serde_json::from_str(include_str!("../input-prefill.json")).unwrap();
        let requests = parse_requests(&input, &spec).unwrap();
        assert_eq!(requests.len(), 1);
    }
}
