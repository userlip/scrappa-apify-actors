use serde_json::Value;

use crate::{
    input::TranslationRequest,
    results::{
        TranslationDatasetItem, build_translation_dataset_item, build_translation_failure_item,
        build_translation_failure_item_with_message,
    },
    scrappa::ScrappaError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushTranslationResult {
    pub saved: bool,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationRunSummary {
    pub requested: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub saved: usize,
    pub status_message: Option<String>,
    pub first_item: Option<TranslationDatasetItem>,
}

pub trait TranslationRunner {
    async fn translate(&self, request: &TranslationRequest) -> Result<Value, ScrappaError>;
}

pub trait TranslationOutput {
    fn charge_limit_status(&self, processed: usize, requested: usize) -> Option<String>;

    async fn push_translation_result(
        &mut self,
        item: &TranslationDatasetItem,
    ) -> Result<PushTranslationResult, String>;
}

pub async fn run_translations<R, O>(
    requests: &[TranslationRequest],
    runner: &R,
    output: &mut O,
) -> Result<TranslationRunSummary, String>
where
    R: TranslationRunner,
    O: TranslationOutput,
{
    let mut succeeded = 0;
    let mut failed = 0;
    let mut saved = 0;
    let mut status_message = None;
    let mut first_item = None;

    for request in requests {
        status_message = output.charge_limit_status(saved, requests.len());
        if let Some(message) = &status_message {
            println!("{message}");
            break;
        }

        println!(
            "Translating item {}/{} from {} to {}",
            request.index + 1,
            requests.len(),
            request.source,
            request.target
        );

        let item = match runner.translate(request).await {
            Ok(response) => match build_translation_dataset_item(request, &response) {
                Ok(item) => item,
                Err(message) => build_translation_failure_item_with_message(request, message),
            },
            Err(error) if error.is_authentication_error() => return Err(error.to_string()),
            Err(error) => {
                eprintln!("Translation item {} failed: {error}", request.index + 1);
                build_translation_failure_item(request, &error)
            }
        };

        let push_result = output.push_translation_result(&item).await?;
        if !push_result.saved {
            status_message = Some(push_result.status_message.unwrap_or_else(|| {
                "Charge limit reached before saving all successful translation results.".to_owned()
            }));
            break;
        }

        if first_item.is_none() {
            first_item = Some(item.clone());
        }
        saved += 1;
        if item.success {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }

    Ok(TranslationRunSummary {
        requested: requests.len(),
        succeeded,
        failed,
        saved,
        status_message,
        first_item,
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Mutex};

    use serde_json::json;

    use crate::{input::build_translation_requests, results::TranslationDatasetItem};

    use super::*;

    struct FakeRunner {
        responses: Mutex<VecDeque<Result<Value, ScrappaError>>>,
        requests: Mutex<Vec<TranslationRequest>>,
    }

    impl FakeRunner {
        fn new(responses: Vec<Result<Value, ScrappaError>>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl TranslationRunner for FakeRunner {
        async fn translate(&self, request: &TranslationRequest) -> Result<Value, ScrappaError> {
            self.requests.lock().unwrap().push(request.clone());
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(json!({"translated_text":"unused"})))
        }
    }

    struct FakeOutput {
        items: Vec<TranslationDatasetItem>,
        limit_before: Option<usize>,
        push_results: VecDeque<PushTranslationResult>,
    }

    impl FakeOutput {
        fn new() -> Self {
            Self {
                items: Vec::new(),
                limit_before: None,
                push_results: VecDeque::new(),
            }
        }
    }

    impl TranslationOutput for FakeOutput {
        fn charge_limit_status(&self, processed: usize, requested: usize) -> Option<String> {
            self.limit_before
                .filter(|limit| processed >= *limit)
                .map(|_| format!("Charge limit reached before fetching; {processed} of {requested} processed."))
        }

        async fn push_translation_result(
            &mut self,
            item: &TranslationDatasetItem,
        ) -> Result<PushTranslationResult, String> {
            self.items.push(item.clone());
            Ok(self
                .push_results
                .pop_front()
                .unwrap_or(PushTranslationResult {
                    saved: true,
                    status_message: None,
                }))
        }
    }

    #[tokio::test]
    async fn continues_a_batch_after_an_item_fails() {
        let requests = build_translation_requests(&json!({
            "items": [
                {"text":"Good morning", "source":"en", "target":"de"},
                {"text":"How are you?", "source":"en", "target":"es"},
                {"text":"Thank you", "source":"en", "target":"fr"}
            ]
        }))
        .unwrap();
        let runner = FakeRunner::new(vec![
            Ok(json!({"translated_text":"Good morning translated"})),
            Err(ScrappaError::Http {
                status: 503,
                details: "Translation service temporarily unavailable. Please retry.".to_owned(),
            }),
            Ok(json!({"translated_text":"Thank you translated"})),
        ]);
        let mut output = FakeOutput::new();

        let summary = run_translations(&requests, &runner, &mut output)
            .await
            .unwrap();

        assert_eq!(runner.requests.lock().unwrap().len(), 3);
        assert_eq!(summary.requested, 3);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.saved, 3);
        assert_eq!(summary.status_message, None);
        assert_eq!(output.items[1].status_code, Some(503));
    }

    #[tokio::test]
    async fn stops_before_fetching_when_the_translation_charge_limit_is_exhausted() {
        let requests = build_translation_requests(&json!({
            "items": [
                {"text":"Good morning", "source":"en", "target":"de"},
                {"text":"How are you?", "source":"en", "target":"es"}
            ]
        }))
        .unwrap();
        let runner = FakeRunner::new(vec![]);
        let mut output = FakeOutput::new();
        output.limit_before = Some(0);

        let summary = run_translations(&requests, &runner, &mut output)
            .await
            .unwrap();

        assert!(runner.requests.lock().unwrap().is_empty());
        assert_eq!(summary.saved, 0);
        assert_eq!(
            summary.status_message.as_deref(),
            Some("Charge limit reached before fetching; 0 of 2 processed.")
        );
    }

    #[tokio::test]
    async fn aborts_a_batch_on_scrappa_authentication_errors() {
        let requests = build_translation_requests(&json!({
            "items": [
                {"text":"Good morning", "source":"en", "target":"de"},
                {"text":"How are you?", "source":"en", "target":"es"}
            ]
        }))
        .unwrap();
        let runner = FakeRunner::new(vec![Err(ScrappaError::Http {
            status: 401,
            details: "Unauthenticated.".to_owned(),
        })]);
        let mut output = FakeOutput::new();

        assert_eq!(
            run_translations(&requests, &runner, &mut output)
                .await
                .unwrap_err(),
            "Scrappa API error (401): Unauthenticated."
        );
        assert!(output.items.is_empty());
    }

    #[tokio::test]
    async fn stops_after_a_result_cannot_be_saved() {
        let requests = build_translation_requests(&json!({
            "items": [
                {"text":"Good morning", "source":"en", "target":"de"},
                {"text":"How are you?", "source":"en", "target":"es"}
            ]
        }))
        .unwrap();
        let runner = FakeRunner::new(vec![]);
        let mut output = FakeOutput::new();
        output.push_results.push_back(PushTranslationResult {
            saved: false,
            status_message: Some("Charge limit reached during push.".to_owned()),
        });

        let summary = run_translations(&requests, &runner, &mut output)
            .await
            .unwrap();

        assert_eq!(runner.requests.lock().unwrap().len(), 1);
        assert_eq!(summary.saved, 0);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(
            summary.status_message.as_deref(),
            Some("Charge limit reached during push.")
        );
    }
}
