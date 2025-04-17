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
    Waiting,
    Running,
    Error,
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
