use super::Artifact;
use super::Scenario;

impl Artifact for Scenario {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Scenario {
    pub fn get_conditions(&self) -> Option<Condition> {
        self.spec.condition.clone()
    }

    pub fn get_actions(&self) -> String {
        self.spec.action.clone()
    }

    pub fn get_targets(&self) -> String {
        self.spec.target.clone()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ScenarioSpec {
    condition: Option<Condition>,
    action: String,
    target: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ScenarioStatus {
    state: ScenarioState,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
enum ScenarioState {
    None,
    Idle,
    Waiting,
    Satisfied,
    Allowed,
    Denied,
    Completed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Condition {
    express: String,
    value: String,
    operands: Operand,
}

impl Condition {
    pub fn get_express(&self) -> String {
        self.express.clone()
    }

    pub fn get_value(&self) -> String {
        self.value.clone()
    }

    pub fn get_operand_value(&self) -> String {
        self.operands.value.clone()
    }

    pub fn get_operand_name(&self) -> String {
        self.operands.name.clone()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct Operand {
    r#type: String,
    name: String,
    value: String,
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MetaData;

    fn create_test_scenario() -> Scenario {
        Scenario {
            apiVersion: "v1".to_string(),
            kind: "Scenario".to_string(),
            metadata: MetaData {
                name: "test-scenario".to_string(),
                labels: None,
                annotations: None,
            },
            spec: ScenarioSpec {
                condition: Some(Condition {
                    express: "eq".to_string(),
                    value: "ready".to_string(),
                    operands: Operand {
                        r#type: "pod".to_string(),
                        name: "test-pod".to_string(),
                        value: "status".to_string(),
                    },
                }),
                action: "start".to_string(),
                target: "model-1".to_string(),
            },
            status: Some(ScenarioStatus {
                state: ScenarioState::None,
            }),
        }
    }

    #[test]
    fn test_artifact_trait_implementation() {
        let scenario = create_test_scenario();
        assert_eq!(scenario.get_name(), "test-scenario");
    }

    #[test]
    fn test_get_conditions() {
        let scenario = create_test_scenario();
        let conditions = scenario.get_conditions().unwrap();

        assert_eq!(conditions.get_express(), "eq");
        assert_eq!(conditions.get_value(), "ready");
        assert_eq!(conditions.get_operand_name(), "test-pod");
        assert_eq!(conditions.get_operand_value(), "status");
    }

    #[test]
    fn test_get_actions() {
        let scenario = create_test_scenario();
        assert_eq!(scenario.get_actions(), "start");
    }

    #[test]
    fn test_get_targets() {
        let scenario = create_test_scenario();
        assert_eq!(scenario.get_targets(), "model-1");
    }

    #[test]
    fn test_scenario_without_conditions() {
        let scenario = Scenario {
            apiVersion: "v1".to_string(),
            kind: "Scenario".to_string(),
            metadata: MetaData {
                name: "no-condition-scenario".to_string(),
                labels: None,
                annotations: None,
            },
            spec: ScenarioSpec {
                condition: None,
                action: "stop".to_string(),
                target: "model-2".to_string(),
            },
            status: None,
        };

        assert!(scenario.get_conditions().is_none());
        assert_eq!(scenario.get_actions(), "stop");
        assert_eq!(scenario.get_targets(), "model-2");
    }

    #[test]
    fn test_scenario_status_states() {
        let waiting_status = ScenarioStatus {
            state: ScenarioState::Waiting,
        };
        let satisfied_status = ScenarioStatus {
            state: ScenarioState::Satisfied,
        };
        let allowed_status = ScenarioStatus {
            state: ScenarioState::Allowed,
        };
        let completed_status = ScenarioStatus {
            state: ScenarioState::Completed,
        };
        let none_status = ScenarioStatus {
            state: ScenarioState::None,
        };

        // Just verify they can be created and pattern matched
        match waiting_status.state {
            ScenarioState::Waiting => assert!(true),
            _ => assert!(false, "Incorrect state"),
        }

        match satisfied_status.state {
            ScenarioState::Satisfied => assert!(true),
            _ => assert!(false, "Incorrect state"),
        }

        match allowed_status.state {
            ScenarioState::Allowed => assert!(true),
            _ => assert!(false, "Incorrect state"),
        }

        match completed_status.state {
            ScenarioState::Completed => assert!(true),
            _ => assert!(false, "Incorrect state"),
        }

        match none_status.state {
            ScenarioState::None => assert!(true),
            _ => assert!(false, "Incorrect state"),
        }
    }

    #[test]
    fn test_scenario_spec_serialization() {
        let spec = ScenarioSpec {
            condition: Some(Condition {
                express: "gt".to_string(),
                value: "5".to_string(),
                operands: Operand {
                    r#type: "metric".to_string(),
                    name: "cpu_usage".to_string(),
                    value: "value".to_string(),
                },
            }),
            action: "scale".to_string(),
            target: "deployment".to_string(),
        };

        let serialized = serde_json::to_string(&spec).unwrap();
        let deserialized: ScenarioSpec = serde_json::from_str(&serialized).unwrap();

        assert_eq!(spec, deserialized);
    }

    #[test]
    fn test_condition_cloning() {
        let condition = Condition {
            express: "lt".to_string(),
            value: "10".to_string(),
            operands: Operand {
                r#type: "metric".to_string(),
                name: "memory_usage".to_string(),
                value: "value".to_string(),
            },
        };

        let cloned = condition.clone();
        assert_eq!(condition, cloned);
    }
}
