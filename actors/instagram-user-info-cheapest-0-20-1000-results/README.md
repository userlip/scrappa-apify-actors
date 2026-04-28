# Instagram User Info

Get public Instagram profile data by username, including biography, follower counts, verification status, profile picture, and related profile fields.

## Input

```json
{
  "username": "natgeo"
}
```

Use the username without the `@` symbol. The actor also accepts usernames with `@` and normalizes them before sending the request.

## Output

Each run pushes one profile object to the default dataset.
