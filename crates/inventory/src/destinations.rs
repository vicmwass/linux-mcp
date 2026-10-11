//! Deny-by-default outbound destination policy.

use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DestinationError {
    #[error("destination policy denied the port")]
    PortDenied,
    #[error("DNS returned no usable addresses")]
    EmptyResolution,
    #[error("destination address is prohibited")]
    AddressDenied,
    #[error("selected address was not in the validated DNS result")]
    SelectionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEvidence {
    pub hostname: String,
    pub port: u16,
    pub addresses: Vec<IpAddr>,
    pub policy_revision: u64,
}

#[derive(Debug, Clone, Default)]
pub struct DestinationPolicy {
    pub allowed_ports: BTreeSet<u16>,
    pub allow_private: bool,
    pub allowed_networks: Vec<Cidr>,
    pub policy_revision: u64,
}

impl DestinationPolicy {
    pub fn evaluate(
        &self,
        hostname: &str,
        port: u16,
        addresses: &[IpAddr],
    ) -> Result<PolicyEvidence, DestinationError> {
        if !self.allowed_ports.is_empty() && !self.allowed_ports.contains(&port) {
            return Err(DestinationError::PortDenied);
        }
        if addresses.is_empty() {
            return Err(DestinationError::EmptyResolution);
        }
        if addresses.iter().any(|ip| !self.address_allowed(*ip)) {
            return Err(DestinationError::AddressDenied);
        }
        Ok(PolicyEvidence {
            hostname: hostname.to_owned(),
            port,
            addresses: addresses.to_vec(),
            policy_revision: self.policy_revision,
        })
    }

    pub fn validate_selected(
        &self,
        evidence: &PolicyEvidence,
        selected: IpAddr,
    ) -> Result<(), DestinationError> {
        if evidence.addresses.contains(&selected) && self.address_allowed(selected) {
            Ok(())
        } else {
            Err(DestinationError::SelectionMismatch)
        }
    }

    fn address_allowed(&self, ip: IpAddr) -> bool {
        let explicit = self
            .allowed_networks
            .iter()
            .any(|network| network.contains(ip));
        if is_blocked_infrastructure(ip) && !(self.allow_private && explicit) {
            return false;
        }
        explicit || !is_private_or_special(ip)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix: u8,
}

impl Cidr {
    pub fn new(network: IpAddr, prefix: u8) -> Option<Self> {
        let max = if network.is_ipv4() { 32 } else { 128 };
        (prefix <= max).then_some(Self { network, prefix })
    }
    pub fn contains(self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(n), IpAddr::V4(i)) => mask4(n, self.prefix) == mask4(i, self.prefix),
            (IpAddr::V6(n), IpAddr::V6(i)) => mask6(n, self.prefix) == mask6(i, self.prefix),
            _ => false,
        }
    }
}

fn mask4(ip: Ipv4Addr, prefix: u8) -> u32 {
    let bits = u32::from(ip);
    if prefix == 0 {
        0
    } else {
        bits & (!0u32 << (32 - prefix))
    }
}
fn mask6(ip: Ipv6Addr, prefix: u8) -> u128 {
    let bits = u128::from(ip);
    if prefix == 0 {
        0
    } else {
        bits & (!0u128 << (128 - prefix))
    }
}
fn is_private_or_special(ip: IpAddr) -> bool {
    is_blocked_infrastructure(ip) || ip.is_loopback() || ip.is_unspecified()
}
fn is_blocked_infrastructure(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            v.is_private()
                || v.is_link_local()
                || v.is_broadcast()
                || v.is_multicast()
                || v.octets() == [169, 254, 169, 254]
                || v.octets()[0] == 192 && v.octets()[1] == 0 && v.octets()[2] == 2
        }
        IpAddr::V6(v) => {
            v.is_unique_local()
                || v.is_unicast_link_local()
                || v.is_multicast()
                || (v.segments()[0] == 0x2001 && v.segments()[1] == 0x0db8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn denies_private_and_metadata_by_default() {
        let p = DestinationPolicy {
            allowed_ports: [22].into_iter().collect(),
            ..Default::default()
        };
        assert_eq!(
            p.evaluate("x", 22, &["127.0.0.1".parse().unwrap()]),
            Err(DestinationError::AddressDenied)
        );
        assert_eq!(
            p.evaluate("x", 22, &["169.254.169.254".parse().unwrap()]),
            Err(DestinationError::AddressDenied)
        );
    }

    #[test]
    fn explicit_private_cidr_requires_self_hosted_allowance_and_rechecks_selection() {
        let network = Cidr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 8).unwrap();
        let p = DestinationPolicy {
            allowed_ports: [22].into_iter().collect(),
            allow_private: true,
            allowed_networks: vec![network],
            policy_revision: 4,
        };
        let e = p.evaluate(
            "internal",
            22,
            &["10.2.3.4".parse().unwrap(), "2001:db8::1".parse().unwrap()],
        );
        assert_eq!(e, Err(DestinationError::AddressDenied));
        let e = p
            .evaluate("internal", 22, &["10.2.3.4".parse().unwrap()])
            .unwrap();
        assert!(p.validate_selected(&e, "10.2.3.4".parse().unwrap()).is_ok());
        assert_eq!(
            p.validate_selected(&e, "10.2.3.5".parse().unwrap()),
            Err(DestinationError::SelectionMismatch)
        );
        assert!(
            !Cidr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 128)
                .unwrap()
                .contains("::2".parse().unwrap())
        );
    }
}
