use crate::commands::{Operation, RiskLevel};

#[derive(Debug, Clone)]
pub struct ExecutionAnalysis {
    pub operations: Vec<Operation>,
    pub risk: RiskLevel,
    pub requires_sudo: bool,
    pub network_access: bool,
    pub destructive: bool,
}

impl ExecutionAnalysis {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            risk: RiskLevel::Low,
            requires_sudo: false,
            network_access: false,
            destructive: false,
        }
    }

    pub fn add_operation(&mut self, operation: Operation) {
        let risk = operation.risk_level();
        let requires_sudo = operation.requires_sudo();
        let network_access = matches!(operation, Operation::NetworkRead);
        let destructive = matches!(
            &operation,
            Operation::FileDelete
                | Operation::UserDelete
                | Operation::GroupDelete
                | Operation::PackageRemove
                | Operation::ServiceStop
                | Operation::SystemReboot
                | Operation::SystemShutdown
        );

        if !self.operations.contains(&operation) {
            self.operations.push(operation);
        }

        self.risk = highest_risk(&self.risk, &risk);

        if requires_sudo {
            self.requires_sudo = true;
        }

        if network_access {
            self.network_access = true;
        }

        if destructive {
            self.destructive = true;
        }
    }
}

impl Default for ExecutionAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

pub fn analyze(operation: Operation) -> ExecutionAnalysis {
    let mut analysis = ExecutionAnalysis::new();
    analysis.add_operation(operation);
    analysis
}

fn highest_risk(
    current: &RiskLevel,
    candidate: &RiskLevel,
) -> RiskLevel {
    if risk_value(candidate) > risk_value(current) {
        candidate.clone()
    } else {
        current.clone()
    }
}

fn risk_value(risk: &RiskLevel) -> u8 {
    match risk {
        RiskLevel::Low => 1,
        RiskLevel::Medium => 2,
        RiskLevel::High => 3,
        RiskLevel::Critical => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_read_only_command() {
        let result = analyze(Operation::ReadOnly);

        assert_eq!(
            result.operations,
            vec![Operation::ReadOnly]
        );

        assert_eq!(result.risk, RiskLevel::Low);
        assert!(!result.requires_sudo);
        assert!(!result.destructive);
    }

    #[test]
    fn analyzes_echo_exit_status_as_read_only() {
        let result = analyze(Operation::ReadOnly);

        assert_eq!(
            result.operations,
            vec![Operation::ReadOnly]
        );

        assert_eq!(result.risk, RiskLevel::Low);
        assert!(!result.requires_sudo);
        assert!(!result.destructive);
    }

    #[test]
    fn analyzes_service_restart() {
        let result = analyze(Operation::ServiceRestart);

        assert_eq!(
            result.operations,
            vec![Operation::ServiceRestart]
        );

        assert_eq!(result.risk, RiskLevel::Medium);
        assert!(result.requires_sudo);
    }

    #[test]
    fn analyzes_file_deletion() {
        let result = analyze(Operation::FileDelete);

        assert_eq!(
            result.operations,
            vec![Operation::FileDelete]
        );

        assert_eq!(result.risk, RiskLevel::High);
        assert!(result.destructive);
    }

    #[test]
    fn detects_network_access() {
        let result = analyze(Operation::NetworkRead);

        assert!(
            result.network_access
        );

        assert!(
            result
                .operations
                .contains(&Operation::NetworkRead)
        );
    }

    #[test]
    fn unknown_operations_are_preserved() {
        let result = analyze(Operation::Unknown);

        assert_eq!(
            result.operations,
            vec![Operation::Unknown]
        );
    }
}