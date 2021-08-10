use super::*;
use serde_json::Value;
use std::fs;

fn create_fake_block_device(name: &str) -> BlockDevice {
    BlockDevice {
        name: name.into(),
        maj_min: "".into(),
        rm: true,
        size: "".into(),
        ro: false,
        block_device_type: "disk".into(),
        model: None,
        serial: None,
        wwn: None,
        children: None,
    }
}

#[test]
fn test_from_tests_folder() {
    for file in fs::read_dir("tests")
        .expect("cannot enumerate tests folder")
        .filter(|item| {
            item.as_ref()
                .unwrap()
                .file_name()
                .into_string()
                .expect("cannot convert file name to string")
                .ends_with(".json")
        })
    {
        let file = file.expect("cannot get test file");

        println!("processing {:?}", file);

        let block_device = create_fake_block_device(
            file.file_name()
                .to_str()
                .expect("cannot get test file name"),
        );

        let json: Value = {
            let contents = fs::read_to_string(file.path())
                .expect("Something went wrong reading the test case file");
            serde_json::from_str(&contents).expect("test case is not a valid json")
        };

        let expected_value: i64 = {
            let expected_file_name = file
                .path()
                .to_str()
                .expect("cannot convert test file into a string")
                .replace(".json", ".expected");

            trace!("parsing {}", expected_file_name);

            let value = fs::read_to_string(expected_file_name)
                .expect("Something went wrong reading the test case expected result file");
            let value = value.strip_suffix("\n").unwrap_or_else(|| &value);

            trace!("value {:#?}", value);

            value
                .parse()
                .expect("expected result file must only be an integer")
        };

        assert_eq!(process_rules(&block_device, &json), Some(expected_value));
    }
}
