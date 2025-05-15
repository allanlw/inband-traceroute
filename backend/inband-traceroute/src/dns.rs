use core::{fmt, net::IpAddr};

use hickory_resolver::{
    config::ResolverConfig, name_server::TokioConnectionProvider, Resolver, TokioResolver,
};

pub(crate) struct ReverseDnsProvider {
    resolver: TokioResolver,
}

impl ReverseDnsProvider {
    pub async fn new() -> anyhow::Result<Self> {
        let resolver = Resolver::builder_with_config(
            ResolverConfig::google(),
            TokioConnectionProvider::default(),
        )
        .build();

        Ok(Self { resolver })
    }

    pub async fn reverse_lookup(&self, ip: &IpAddr) -> anyhow::Result<String> {
        let result = self.resolver.reverse_lookup(*ip).await?;

        let answers = result.iter().collect::<Vec<_>>();

        if answers.is_empty() {
            return Err(anyhow::anyhow!("No answer found",));
        }

        return Ok(answers[0].0.to_ascii());
    }
}

impl fmt::Debug for ReverseDnsProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReverseDnsProvider")
    }
}
