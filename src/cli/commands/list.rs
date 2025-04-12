use crate::actions::list;
use crate::{core::repository::Repository, core::settings::Settings, Result};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    repository_path: Option<String>,
    repository_url: Option<String>,
}

pub(crate) async fn do_raw(
    params: &list::Parameters,
    _repo: &dyn Repository,
) -> Result<list::Result, list::Error> {
    list::run(params)
}

pub(crate) async fn do_cli(
    args: ListArgs,
    settings: &Settings,
    repo: &dyn Repository,
) -> Result<list::Result, list::Error> {
    let parameters = make_parameters(args, settings).await?;
    do_raw(&parameters, repo).await
}

pub(crate) async fn make_parameters(
    args: ListArgs,
    settings: &Settings,
) -> Result<list::Parameters, list::Error> {
    let result = list::Parameters {
        repository_path: args
            .repository_path
            .unwrap_or(settings.global_repository_path.clone()),
        repository_url: args
            .repository_url
            .unwrap_or(settings.repository_url.clone()),
    };
    Ok(result)
}

pub(crate) fn from_json(json: &str) -> Result<list::Parameters> {
    let result = serde_json::from_str::<list::Parameters>(json)
        .map_err(|e| miette::Report::msg(e.to_string()))?;

    Ok(result)
}
