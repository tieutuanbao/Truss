use sha2::{Digest, Sha256};

use crate::application::{CoreDistributionPort, PortError};
use crate::domain::{ContentHash, CoreDistribution, DistributionFile, RelativePath};

#[derive(Clone, Copy, Default)]
pub struct EmbeddedCoreDistribution;

impl CoreDistributionPort for EmbeddedCoreDistribution {
    fn current(&self) -> Result<CoreDistribution, PortError> {
        let mut files = Vec::new();
        let agent_block = include_bytes!("../../../../scripts/agent-truss-block.md");
        let mut agents = b"# Agent Instructions\n\n".to_vec();
        agents.extend_from_slice(agent_block);
        add(&mut files, "AGENTS.md", &agents)?;
        add(
            &mut files,
            ".agents/skills/audit-onboarding-proposal/SKILL.md",
            include_bytes!("../../../../.agents/skills/audit-onboarding-proposal/SKILL.md"),
        )?;
        add(
            &mut files,
            ".agents/skills/audit-onboarding-proposal/agents/openai.yaml",
            include_bytes!(
                "../../../../.agents/skills/audit-onboarding-proposal/agents/openai.yaml"
            ),
        )?;
        add(
            &mut files,
            ".agents/skills/audit-onboarding-proposal/scripts/validate_evidence_capsule.py",
            include_bytes!(
                "../../../../.agents/skills/audit-onboarding-proposal/scripts/validate_evidence_capsule.py"
            ),
        )?;
        add(
            &mut files,
            ".agents/skills/encode-invariant/SKILL.md",
            include_bytes!("../../../../.agents/skills/encode-invariant/SKILL.md"),
        )?;
        add(
            &mut files,
            ".agents/skills/encode-invariant/agents/openai.yaml",
            include_bytes!("../../../../.agents/skills/encode-invariant/agents/openai.yaml"),
        )?;
        add(
            &mut files,
            ".agents/skills/improve-truss/SKILL.md",
            include_bytes!("../../../../.agents/skills/improve-truss/SKILL.md"),
        )?;
        add(
            &mut files,
            ".agents/skills/improve-truss/agents/openai.yaml",
            include_bytes!("../../../../.agents/skills/improve-truss/agents/openai.yaml"),
        )?;
        add(
            &mut files,
            ".agents/skills/onboard-repository/SKILL.md",
            include_bytes!("../../../../.agents/skills/onboard-repository/SKILL.md"),
        )?;
        add(
            &mut files,
            ".agents/skills/onboard-repository/agents/openai.yaml",
            include_bytes!("../../../../.agents/skills/onboard-repository/agents/openai.yaml"),
        )?;
        add(
            &mut files,
            ".agents/skills/onboard-repository/references/evidence-capsule-v1.md",
            include_bytes!(
                "../../../../.agents/skills/onboard-repository/references/evidence-capsule-v1.md"
            ),
        )?;
        add(
            &mut files,
            ".agents/skills/onboard-repository/references/evidence-capsule-v2.md",
            include_bytes!(
                "../../../../.agents/skills/onboard-repository/references/evidence-capsule-v2.md"
            ),
        )?;
        add(
            &mut files,
            ".agents/skills/onboard-repository/scripts/emit_evidence_bundle.py",
            include_bytes!(
                "../../../../.agents/skills/onboard-repository/scripts/emit_evidence_bundle.py"
            ),
        )?;
        add(
            &mut files,
            ".agents/skills/onboard-repository/scripts/render_patch.py",
            include_bytes!("../../../../.agents/skills/onboard-repository/scripts/render_patch.py"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/WORKFLOW.md",
            include_bytes!("../../../../.truss-core/docs/WORKFLOW.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/communication.md",
            include_bytes!("../../assets/.truss-core/docs/communication.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/README.md",
            include_bytes!("../../../../.truss-core/docs/README.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/patterns/encoding-invariants.md",
            include_bytes!("../../../../.truss-core/docs/patterns/encoding-invariants.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/product/README.md",
            include_bytes!("../../../../.truss-core/docs/product/README.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/plans/README.md",
            include_bytes!("../../assets/.truss-core/docs/plans/README.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/plans/active/README.md",
            include_bytes!("../../../../.truss-core/docs/plans/active/README.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/plans/completed/README.md",
            include_bytes!("../../assets/.truss-core/docs/plans/completed/README.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/decisions/README.md",
            include_bytes!("../../assets/.truss-core/docs/decisions/README.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/templates/application-runbook.md",
            include_bytes!("../../../../.truss-core/docs/templates/application-runbook.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/templates/decision.md",
            include_bytes!("../../../../.truss-core/docs/templates/decision.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/templates/exec-plan.md",
            include_bytes!("../../../../.truss-core/docs/templates/exec-plan.md"),
        )?;
        add(
            &mut files,
            ".truss-core/docs/templates/truss-improvement.md",
            include_bytes!("../../../../.truss-core/docs/templates/truss-improvement.md"),
        )?;
        Ok(CoreDistribution {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            files,
        })
    }
}

fn add(files: &mut Vec<DistributionFile>, path: &str, content: &[u8]) -> Result<(), PortError> {
    let path = RelativePath::parse(path).map_err(|error| PortError::new(error.to_string()))?;
    let hash = ContentHash::parse(format!("{:x}", Sha256::digest(content)))
        .map_err(|error| PortError::new(error.to_string()))?;
    files.push(DistributionFile {
        path,
        content: content.to_vec(),
        hash,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_payload_is_generic_and_complete() {
        let distribution = EmbeddedCoreDistribution.current().unwrap();
        distribution.validate().unwrap();
        let embedded_paths = distribution
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let manifest_paths = include_str!("../../../../scripts/truss-install-files.txt")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(embedded_paths, manifest_paths);
        let agents = distribution
            .files
            .iter()
            .find(|file| file.path.as_str() == "AGENTS.md")
            .unwrap();
        let agents = String::from_utf8(agents.content.clone()).unwrap();
        assert!(agents.contains("No control-plane operation is required."));
        assert!(!agents.contains("Current Upstream Goal"));
        let plans = distribution
            .files
            .iter()
            .find(|file| file.path.as_str() == ".truss-core/docs/plans/README.md")
            .unwrap();
        assert!(!String::from_utf8_lossy(&plans.content).contains("rust-truss-core"));
        for skill in [
            ".agents/skills/onboard-repository/SKILL.md",
            ".agents/skills/audit-onboarding-proposal/SKILL.md",
            ".agents/skills/encode-invariant/SKILL.md",
            ".agents/skills/improve-truss/SKILL.md",
        ] {
            let skill = distribution
                .files
                .iter()
                .find(|file| file.path.as_str() == skill)
                .unwrap();
            let skill = String::from_utf8_lossy(&skill.content);
            assert!(skill.contains("authorized") || skill.contains("read-only"));
        }
        for metadata in [
            ".agents/skills/onboard-repository/agents/openai.yaml",
            ".agents/skills/audit-onboarding-proposal/agents/openai.yaml",
            ".agents/skills/improve-truss/agents/openai.yaml",
        ] {
            let metadata = distribution
                .files
                .iter()
                .find(|file| file.path.as_str() == metadata)
                .unwrap();
            assert!(String::from_utf8_lossy(&metadata.content)
                .contains("allow_implicit_invocation: false"));
        }
        let invariant_metadata = distribution
            .files
            .iter()
            .find(|file| file.path.as_str() == ".agents/skills/encode-invariant/agents/openai.yaml")
            .unwrap();
        assert!(String::from_utf8_lossy(&invariant_metadata.content)
            .contains("allow_implicit_invocation: true"));
    }
}
