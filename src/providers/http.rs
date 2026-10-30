use crate::error::{Error,Result};
use std::time::Duration;
pub(crate) fn client()->std::result::Result<reqwest::Client,&'static str>{
    let _=rustls::crypto::ring::default_provider().install_default();
    let roots=webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().map(|der|reqwest::Certificate::from_der(der.as_ref()).map_err(|_|"trust-root")).collect::<std::result::Result<Vec<_>,_>>()?;
    reqwest::Client::builder().tls_certs_only(roots).no_proxy().redirect(reqwest::redirect::Policy::none()).retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(60)).connection_verbose(false).build().map_err(|_|"client-configuration")
}
pub fn endpoint(value:&str,path:&str)->Result<reqwest::Url>{
    let url=reqwest::Url::parse(value).map_err(|_|Error::Invalid)?;
    let host=url.host_str().ok_or(Error::Invalid)?.trim_matches(['[',']']);
    let loopback=host.parse::<std::net::IpAddr>().is_ok_and(|ip|ip.is_loopback());
    if !(url.scheme()=="https"||(url.scheme()=="http"&&loopback)){return Err(Error::Unsupported);}
    if !url.username().is_empty()||url.password().is_some()||url.query().is_some()||url.fragment().is_some()||url.path()!=path||url.port_or_known_default().is_none(){return Err(Error::Invalid);}Ok(url)
}
