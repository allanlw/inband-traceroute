use anyhow::{anyhow, Context as _};
use aya_build::cargo_metadata;

fn main() -> anyhow::Result<()> {
    let cargo_metadata::Metadata { packages, .. } = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .context("MetadataCommand::exec")?;
    let ebpf_package = packages
        .into_iter()
        .find(|cargo_metadata::Package { name, .. }| name == "inband-traceroute-ebpf")
        .ok_or_else(|| anyhow!("inband-traceroute-ebpf package not found"))?;
    aya_build::build_ebpf(
        [ebpf_package],
        aya_build::Toolchain::Nightly, // TODO: revert to nightly after https://github.com/rust-lang/compiler-builtins/issues/908 is fixed
    )
}
