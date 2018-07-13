//! This model handles grouping messages given on ther classification and
//! the first level of traffic shaping.

pub mod boolean;
pub mod bucket;
mod utils;

pub use self::utils::{Grouper, MaybeMessage};
use classifier::Classified;
use error::TSError;

use prometheus::Counter;

lazy_static! {
    /*
     * Number of messages that pass the grouping stage.
     */
    static ref GROUPING_PASS: Counter =
        register_counter!(opts!("ts_grouping_pass", "Passes during the grouping stage.")).unwrap();
    /*
     * Number of messages marked to drop the grouping stage.
     */
    static ref GROUPING_DROP: Counter =
        register_counter!(opts!("ts_grouping_drop", "Drops during the grouping stage.")).unwrap();
    /*
     * Errors during the grouping stage
     */
    static ref GROUPING_ERR: Counter =
        register_counter!(opts!("ts_grouping_error", "Errors during the grouping stage.")).unwrap();
}

pub fn new<'a>(name: &'a str, opts: &'a str) -> Groupers<'a> {
    match name {
        "drop" => Groupers::Boolean(boolean::Grouper::new("true")),
        "pass" => Groupers::Boolean(boolean::Grouper::new("false")),
        "bucket" => Groupers::Bucket(bucket::Grouper::new(opts)),

        _ => panic!(
            "Unknown grouper: {} valid options are 'bucket', 'drop' and 'pass'",
            name
        ),
    }
}

pub enum Groupers<'a> {
    Boolean(boolean::Grouper),
    Bucket(bucket::Grouper<'a>),
}
impl<'a> Grouper for Groupers<'a> {
    fn group<'p, 'c: 'p>(&mut self, msg: Classified<'p, 'c>) -> Result<MaybeMessage<'p>, TSError> {
        let r = match self {
            Groupers::Boolean(g) => g.group(msg),
            Groupers::Bucket(g) => g.group(msg),
        };
        match r {
            Err(_) => GROUPING_ERR.inc(),
            Ok(MaybeMessage { drop: false, .. }) => GROUPING_PASS.inc(),
            Ok(MaybeMessage { drop: true, .. }) => GROUPING_DROP.inc(),
        };
        r
    }
}

#[cfg(test)]
mod tests {
    use classifier;
    use classifier::Classifier;
    use grouping;
    use grouping::Grouper;
    use parser;
    use parser::Parser;
    #[test]
    fn boolean_grouper() {
        let s = "Example";
        let p = parser::new("raw", "");
        let c = classifier::new("constant", "Classification");
        let mut g_d = grouping::new("drop", "");
        let mut g_k = grouping::new("pass", "");

        let r = p.parse(s)
            .and_then(|parsed| c.classify(parsed))
            .and_then(|classified| g_d.group(classified))
            .expect("grouping failed");
        assert_eq!(r.drop, true);

        let r = p.parse(s)
            .and_then(|parsed| c.classify(parsed))
            .and_then(|classified| g_k.group(classified))
            .expect("grouping failed");
        assert_eq!(r.drop, false);
    }

}