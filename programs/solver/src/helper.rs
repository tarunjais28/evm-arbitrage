use super::*;

pub fn get_events(
    contract: &Contract<WebSocket>,
    names: &[&str],
) -> Result<(Vec<Event>, Vec<H256>), anyhow::Error> {
    let mut events = Vec::with_capacity(names.len());
    let mut signatures = Vec::with_capacity(names.len());

    for name in names {
        let event = contract
            .abi()
            .events_by_name(name)?
            .first()
            .ok_or_else(|| anyhow::anyhow!("Event not found: {}", name))?
            .clone(); // Clone because it's a reference

        let signature = event.signature();

        events.push(event);
        signatures.push(signature);
    }

    Ok((events, signatures))
}
