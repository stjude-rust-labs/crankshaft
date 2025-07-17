use clap::Parser as Clap;
use clap::ValueHint;
use tonic::transport::Uri;

#[derive(Clap, Debug)]
#[clap(
    name = clap::crate_name!()
)]
pub struct Config {
    #[clap(value_hint = ValueHint::Url)]
    pub target_addr: Option<Uri>,
}
