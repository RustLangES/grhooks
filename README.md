# GRHooks - Webhook Server Configuration

GRHooks is a configurable webhook server that can execute commands or scripts in response to incoming webhook events. It supports multiple webhook configurations with flexible command execution options.

## Install prebuilt binaries via shell script

```sh
bash <(curl -sSL https://raw.githubusercontent.com/RustLangES/grhooks/main/scripts/install.sh)
```

## Install prebuilt binaries via powershell script

```sh
powershell -ExecutionPolicy Bypass -c "$ProgressPreference='SilentlyContinue'; iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/RustLangES/grhooks/main/scripts/install.ps1'))"
```

## Configuration File

GRHooks uses a configuration file in TOML, YAML, or JSON format. The file should contain the server settings and webhook definitions.

### Example Configuration (TOML)

```toml
port = 8080

[[webhooks]]
path = "deploy"
secret = "mysecret123" # or "${{ env(\"MY_SECRET_FROM_ENV\") }}"
events = ["push", "pull_request"]
command = "echo 'Deployment triggered by ${{event.action}}'"

[[webhooks]]
path = "build"
shell = ["/bin/bash", "-c"]
script = "scripts/build.sh"
events = ["create"]
```

## Configuration Options

### Server Configuration

| Field   | Type   | Description                   | Default | Required |
| ------- | ------ | ----------------------------- | ------- | -------- |
| port    | u16    | Port to listen on             | -       | Yes      |
| verbose | String | Logging verbosity level (0-4) | "0"     | No       |

### Webhook Configuration

Each webhook is defined in the `webhooks` array with the following options:

| Field   | Type                | Description                                                            | Required                             |
| ------- | ------------------- | ---------------------------------------------------------------------- | ------------------------------------ |
| path    | String              | URL path for the webhook                                               | No (defaults to /)                   |
| secret  | Option<String>      | Secret for validating webhook signatures                               | No                                   |
| events  | Vec<String>         | List of events this webhook should handle (use `["*"]` for all events) | Yes                                  |
| shell   | Option<Vec<String>> | Custom shell and arguments to use for command execution                | No (defaults to `/bin/sh -c`)        |
| command | Option<String>      | Command to execute when webhook is triggered                           | Either command or script must be set |
| script  | Option<PathBuf>     | Path to script file to execute when webhook is triggered               | Either command or script must be set |

## Template Variables

Commands and scripts can use template variables that will be replaced with values from the webhook payload. Variables use the syntax `${{path.to.value}}`.

> [!NOTE]
> The template is redered using the [srtemplate](https://github.com/SergioRibera/srtemplate) template engine.

### Common Variables

- `${{event.type}}`: The event type that triggered the webhook

## Running GRHooks

### Command Line Usage

```bash
grhooks [-v...] /path/to/config.[toml|yaml|json]
```

Options:

- `-v`: Increase verbosity (can be used multiple times)

### Environment Variables

- `GRHOOKS_MANIFEST_DIR`: Path to configuration file
- `GRHOOKS_LOG`: Set logging verbosity (0-4 or trace, info, debug, warning, error)

## Webhook Security

When a `secret` is configured in the webhook:

1. GRHooks will validate the `X-Hub-Signature-256` header
2. Only requests with valid signatures will be processed
3. The secret should match what's configured in your Git provider

## Example Use Cases

1. **Deployment Automation**:

   ```toml
   [[webhooks]]
   path = "deploy-prod"
   events = ["deployment"]
   command = "kubectl set image deployment/myapp app=${{event.deployment.payload.image}}"
   ```

2. **CI/CD Pipeline**:

   ```toml
   [[webhooks]]
   path = "build"
   script = "scripts/ci-pipeline.sh"
   events = ["push"]
   ```

3. **Notification System**:
   ```toml
   [[webhooks]]
   path = "notify"
   events = ["*"]
   command = "curl -X POST -d '{\"text\":\"Event ${{event.type}} received\"}' $SLACK_WEBHOOK"
   ```

## Supported Webhook Providers

GRHooks currently supports webhooks from:

- GitHub

Plans to support:

- GitLab
- Bitbucket
