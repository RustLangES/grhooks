use std::path::PathBuf;

use grhooks_config::WebhookConfig;
use srtemplate::SrTemplate;

pub async fn execute_command(
    config: &WebhookConfig,
    event_type: &str,
    value: &serde_json::Value,
) -> std::io::Result<String> {
    let ctx = SrTemplate::with_delimiter("${{", "}}");
    ctx.add_variable("event.type", event_type);
    crate::process_value(&ctx, "event", value);

    let (shell, args) = if let Some(shell) = config.shell.as_ref() {
        let mut args = shell.clone();
        let shell = args.remove(0);
        (shell, args)
    } else {
        ("sh".to_string(), vec!["-c".to_string()])
    };

    let output = if let Some(script_path) = &config.script {
        execute_script(&ctx, script_path, &shell, &args).await?
    } else if let Some(command) = config.command.as_deref() {
        execute_direct_command(&ctx, command, &shell, &args).await?
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No command or script provided",
        ));
    };

    Ok(output)
}

async fn execute_direct_command(
    ctx: &SrTemplate<'_>,
    command: &str,
    shell: &str,
    shell_args: &[String],
) -> std::io::Result<String> {
    let Ok(rendered_cmd) = ctx.render(command.trim()) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to render command",
        ));
    };
    tracing::debug!("Executing command: {}", rendered_cmd);

    let output = tokio::process::Command::new(shell)
        .args(shell_args)
        .arg(&rendered_cmd)
        .output()
        .await?;

    handle_command_output(&output, &rendered_cmd)
}

async fn execute_script(
    ctx: &SrTemplate<'_>,
    script_path: &PathBuf,
    shell: &str,
    shell_args: &[String],
) -> std::io::Result<String> {
    let script_content = std::fs::read_to_string(script_path)?;

    let Ok(rendered_script) = ctx.render(script_content.trim()) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to render script",
        ));
    };

    let temp_script = tempfile::NamedTempFile::new()?;

    std::fs::write(&temp_script, rendered_script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&temp_script)?.permissions();
        perms.set_mode(0o755); // rwxr-xr-x
        std::fs::set_permissions(&temp_script, perms)?;
    }

    tracing::debug!("Executing rendered script: {temp_script:?}");

    let output = tokio::process::Command::new(shell)
        .args(shell_args)
        .arg(temp_script.path())
        .output()
        .await?;

    handle_command_output(&output, &format!("script: {temp_script:?}"))
}

fn handle_command_output(output: &std::process::Output, context: &str) -> std::io::Result<String> {
    if !output.status.success() {
        let err_msg = format!(
            "Command failed ({} - {}):\nSTDERR: {}\nSTDOUT: {}",
            output.status,
            context,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );

        return Err(std::io::Error::new(std::io::ErrorKind::Other, err_msg));
    }

    let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    tracing::debug!("Command Output: {}", output_str);
    Ok(output_str)
}
