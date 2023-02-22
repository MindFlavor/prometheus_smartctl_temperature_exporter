use regex::Regex;

#[derive(Debug, Clone)]
pub(crate) struct Options {
    pub verbose: bool,
    pub prepend_sudo: bool,
    pub exclude_regexes: Vec<Regex>,
}

impl Options {
    pub fn from_claps(
        matches: &clap::ArgMatches<'_>,
    ) -> Result<Options, Box<dyn std::error::Error + Send + Sync>> {
        let exclude_regexes: Result<Vec<_>, _> = matches
            .values_of("exclude_regexes")
            .unwrap_or_default()
            .map(Regex::new)
            .collect();

        let options = Options {
            verbose: matches.is_present("verbose"),
            prepend_sudo: matches.is_present("prepend_sudo"),
            exclude_regexes: exclude_regexes?,
        };

        Ok(options)
    }
}
