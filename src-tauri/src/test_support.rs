use crate::cli::CliProbe;
use crate::types::CliClient;

    pub(crate) fn probe(client: CliClient, version: Option<(u32, u32, u32)>) -> CliProbe {
        let raw = match (client, version) {
            (CliClient::Bd, Some((a, b, c))) => format!("bd version {}.{}.{} (abc123)", a, b, c),
            (CliClient::Br, Some((a, b, c))) => format!("br {}.{}.{} (rustc 1.85.0)", a, b, c),
            _ => "mystery 9.9.9".to_string(),
        };
        CliProbe { client, version, raw }
    }

    pub(crate) fn minimal_issue_json(id: &str, title: &str) -> String {
        format!(
            r#"{{"id":"{}","title":"{}","description":null,"status":"open","priority":3,"issue_type":"task","owner":null,"assignee":null,"labels":[],"created_at":"2025-01-01T00:00:00Z","created_by":null,"updated_at":"2025-01-01T00:00:00Z","closed_at":null,"close_reason":null,"blocked_by":null,"blocks":null,"comments":null,"external_ref":null,"estimate":null,"design":null,"acceptance_criteria":null,"notes":null,"parent":null,"dependents":null,"dependencies":null,"dependency_count":null,"dependent_count":null,"metadata":null,"spec_id":null,"comment_count":null}}"#,
            id, title
        )
    }

    pub(crate) fn issue_json_with_metadata(id: &str, metadata_json: &str) -> String {
        minimal_issue_json(id, "With metadata").replace(r#""metadata":null"#, &format!(r#""metadata":{}"#, metadata_json))
    }
