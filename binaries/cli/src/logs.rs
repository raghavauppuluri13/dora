use dora_core::topics::{ControlRequest, ControlRequestReply};
use eyre::{bail, Context, Result};
use uuid::Uuid;

use crate::control_connection;
use bat::{Input, PrettyPrinter};

pub fn logs(uuid: Option<Uuid>, name: Option<String>, node: String) -> Result<()> {
    let mut control_session = None;
    let connection = control_connection(&mut control_session)?;
    let logs = {
        let reply_raw = connection
            .request(&serde_json::to_vec(&ControlRequest::Logs {
                uuid,
                name,
                node: node.clone(),
            })?)
            .wrap_err("failed to send DaemonConnected message")?;

        let reply = serde_json::from_slice(&reply_raw).wrap_err("failed to parse reply")?;
        match reply {
            ControlRequestReply::Logs { logs } => logs,
            other => bail!("unexpected reply to daemon connection check: {other:?}"),
        }
    };

    PrettyPrinter::new()
        .header(true)
        .grid(true)
        .line_numbers(true)
        .paging_mode(bat::PagingMode::Always)
        .inputs(vec![Input::from_bytes(&logs)
            .name("Logs") // TODO: Make a better name
            .title(format!("Logs from {node}.").as_str())])
        .print()
        .unwrap();

    Ok(())
}
