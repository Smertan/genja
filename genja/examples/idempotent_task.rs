use genja::genja_core::inventory::{BaseBuilderHost, Data, Host, Hosts, Inventory};
use genja::genja_core::task::{
    HostTaskResult, IdempotencyCheck, TaskError, TaskRuntimeContext, TaskSuccess,
};
use genja::{Genja, genja_task};
use serde_json::{Value, json};

struct EnsureNtp;

#[genja_task(name = "ensure_ntp", idempotency = IdempotencyMode::Check)]
impl EnsureNtp {
    async fn check_async(
        &self,
        host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<IdempotencyCheck, TaskError> {
        let desired = desired_ntp_server();
        let configured = host
            .data()
            .and_then(|data| data.get("ntp_server"))
            .and_then(Value::as_str);

        if configured == Some(desired) {
            return Ok(IdempotencyCheck::converged(format!(
                "{desired} is already configured"
            )));
        }

        Ok(
            IdempotencyCheck::change_required(format!("+ntp server {desired}")).with_details(
                json!({
                    "current": configured,
                    "desired": desired,
                }),
            ),
        )
    }

    async fn start_async(
        &self,
        host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        let desired = desired_ntp_server();
        Ok(HostTaskResult::passed(
            TaskSuccess::new()
                .with_changed(true)
                .with_diff(format!("+ntp server {desired}"))
                .with_summary(format!(
                    "configured NTP server on {}",
                    host.hostname().unwrap_or("host")
                )),
        ))
    }
}

fn desired_ntp_server() -> &'static str {
    "192.0.2.10"
}

fn main() -> Result<(), genja::GenjaError> {
    let mut hosts = Hosts::new();
    hosts.add_host(
        "router1",
        Host::builder()
            .hostname("10.0.0.1")
            .data(Data::new(json!({
                "ntp_server": desired_ntp_server(),
            })))
            .build(),
    );
    hosts.add_host(
        "router2",
        Host::builder()
            .hostname("10.0.0.2")
            .data(Data::new(json!({
                "ntp_server": "198.51.100.20",
            })))
            .build(),
    );

    let inventory = Inventory::builder().hosts(hosts).build();
    let genja = Genja::builder(inventory).build()?;
    let results = genja.run_task(EnsureNtp, 1)?;

    let output = results.to_pretty_json_string().map_err(|err| {
        genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
    })?;
    println!("{output}");

    Ok(())
}
