use crate::BlockDevice;
use log::{trace, warn};
use serde_json::Value;
#[cfg(test)]
mod test_engine;

const RULES: &[fn(&BlockDevice, &Value) -> Option<i64>] = &[
    nvme_smart_health_information_log,
    temperature_node,
    ata_smart_attributes_airflow_temperature_cel,
];

pub fn process_rules(block_device: &BlockDevice, json: &Value) -> Option<i64> {
    match RULES.iter().find_map(|rule| rule(block_device, json)) {
        Some(temperature) => {
            trace!(
                "device {:?} reported temperature {}",
                block_device,
                temperature
            );
            Some(temperature)
        }
        None => {
            warn!(
                "device {} is not supported by this tool",
                block_device.signature()
            );
            None
        }
    }
}

fn nvme_smart_health_information_log(block_device: &BlockDevice, json: &Value) -> Option<i64> {
    trace!("checking nvme_smart_health_information_log");

    json.get("nvme_smart_health_information_log")
        .and_then(|nvme_smart| {
            trace!(
                "device {} has nvme_smart_health_information_log node",
                block_device.signature()
            );
            nvme_smart
                .get("temperature")
                .and_then(|temperature| temperature.as_i64())
        })
}

fn temperature_node(_block_device: &BlockDevice, json: &Value) -> Option<i64> {
    trace!("checking temperature_node");

    json.get("temperature").and_then(|temperature| {
        temperature
            .get("current")
            .and_then(|temperature| temperature.as_i64())
    })
}

fn ata_smart_attributes_airflow_temperature_cel(
    _block_device: &BlockDevice,
    json: &Value,
) -> Option<i64> {
    trace!("checking ata_smart_attributes Airflow_Temperature_Cel");

    json.get("ata_smart_attributes")
        .and_then(|ata_smart_attributes| {
            ata_smart_attributes.get("table").and_then(|table| {
                table.as_array().and_then(|table| {
                    table
                        .iter()
                        .find(|item| {
                            item.get("name")
                                .map(|name| name == "Airflow_Temperature_Cel")
                                .unwrap_or_else(|| false)
                        })
                        .and_then(|temperature| {
                            temperature
                                .get("raw")
                                .and_then(|raw| raw.get("value").and_then(|value| value.as_i64()))
                        })
                })
            })
        })
}
