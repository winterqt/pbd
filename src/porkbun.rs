use anyhow::{anyhow, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[cfg(not(test))]
const PORKBUN_API_BASE: &str = "https://porkbun.com/api/json/v3";

pub struct Porkbun {
    api_key: String,
    secret_api_key: String,
    agent: ureq::Agent,
}

impl Porkbun {
    pub fn new(api_key: String, secret_api_key: String) -> Self {
        Porkbun {
            api_key,
            secret_api_key,
            agent: ureq::builder()
                .user_agent(concat!("pbd/", env!("CARGO_PKG_VERSION")))
                .build(),
        }
    }

    fn request<I: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        inner: Option<I>,
    ) -> Result<R> {
        #[cfg(not(test))]
        let base = PORKBUN_API_BASE;

        #[cfg(test)]
        let base = mockito::server_url();

        let url = format!("{}{}", base, path);

        let request = self.agent.request("POST", &url);

        let response: ResponseBody<R> = request
            .send_json(
                serde_json::to_value(RequestBody {
                    api_key: &self.api_key,
                    secret_api_key: &self.secret_api_key,
                    inner,
                })
                .with_context(|| "Failed to serialize body")?,
            )
            .map_err(|e| match e {
                ureq::Error::Status(code, response) => {
                    match response.into_json::<ResponseBody<()>>() {
                        Ok(response) => anyhow!(
                            "{}",
                            response
                                .message
                                .unwrap_or(format!("No message provided (status code: {})", code))
                        ),
                        Err(err) => err.into(),
                    }
                }
                err => err.into(),
            })
            .with_context(|| format!("Failed to request {}", url))?
            .into_json()
            .with_context(|| "Failed to deserialize body")?;

        Ok(response.inner)
    }

    pub fn ping(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct Ping {
            #[serde(rename = "yourIp")]
            ip_address: String,
        }

        Ok(self.request::<(), Ping>("/ping", None)?.ip_address)
    }

    pub fn records(&self, domain: &str) -> Result<Vec<Record>> {
        #[derive(Deserialize)]
        struct Records {
            records: Vec<Record>,
        }

        Ok(self
            .request::<(), Records>(&format!("/dns/retrieve/{}", domain), None)?
            .records)
    }

    pub fn edit_record(&self, domain: &str, record: &Record, content: String) -> Result<()> {
        self.request(
            &format!("/dns/edit/{}/{}", domain, record.id),
            Some(Record {
                content,
                ..record.clone()
            }),
        )?;

        Ok(())
    }

    pub fn create_record(&self, domain: &str, record: &Record) -> Result<()> {
        self.request(&format!("/dns/create/{}", domain), Some(record))?;

        Ok(())
    }
}

#[derive(Serialize)]
struct RequestBody<'a, I> {
    #[serde(rename = "apikey")]
    api_key: &'a str,
    #[serde(rename = "secretapikey")]
    secret_api_key: &'a str,
    #[serde(flatten)]
    inner: Option<I>,
}

#[derive(Deserialize)]
struct ResponseBody<I> {
    message: Option<String>,
    #[serde(flatten)]
    inner: I,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Record {
    #[serde(skip_serializing, with = "what_are_ints")]
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub typ: RecordType,
    pub content: String,
    #[serde(with = "what_are_ints")]
    pub ttl: u32,
    #[serde(rename = "prio", with = "what_are_ints")]
    pub priority: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum RecordType {
    A,
    MX,
    CNAME,
    ALIAS,
    TXT,
    NS,
    AAAA,
    SRV,
    TLSA,
    CAA,
}

mod what_are_ints {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let s = String::deserialize(deserializer)?;

        s.parse().map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(val: &u32, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&val.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, Matcher};
    use serde_json::{json, Value};

    fn client() -> Porkbun {
        Porkbun::new("abc".to_string(), "def".to_string())
    }

    fn body(mut v: Value) -> serde_json::Result<Value> {
        let v = v.as_object_mut().unwrap();

        v.insert("apikey".to_string(), Value::String("abc".to_string()));
        v.insert("secretapikey".to_string(), Value::String("def".to_string()));

        serde_json::to_value(v)
    }

    #[test]
    fn ping() {
        let client = client();

        let m = mock("POST", "/ping")
            .match_body(Matcher::Json(body(json!({})).unwrap()))
            .with_body(
                r#"
        {
            "status": "SUCCESS",
            "yourIp": "1337"
        }
        "#,
            )
            .create();

        let addr = client.ping();

        m.assert();
        assert!(client.ping().is_ok());
        assert_eq!(addr.unwrap(), "1337");
    }

    #[test]
    fn get_records() {
        let client = client();

        let m = mock("POST", "/dns/retrieve/borseth.ink")
            .match_body(Matcher::Json(body(json!({})).unwrap()))
            .with_body(
                json!({
                    "status": "SUCCESS",
                    "records": [
                        {
                            "id": "106926652",
                            "name": "borseth.ink",
                            "type": "A",
                            "content": "1.1.1.1",
                            "ttl": "300",
                            "prio": "0",
                            "notes": ""
                        },
                        {
                            "id": "106926659",
                            "name": "www.borseth.ink",
                            "type": "A",
                            "content": "1.1.1.1",
                            "ttl": "300",
                            "prio": "0",
                            "notes": ""
                        }
                    ]
                })
                .to_string(),
            )
            .create();

        let records = client.records("borseth.ink");

        m.assert();
        assert!(records.is_ok());
        assert_eq!(
            records.unwrap(),
            vec![
                Record {
                    id: 106926652,
                    name: "borseth.ink".to_string(),
                    typ: RecordType::A,
                    content: "1.1.1.1".to_string(),
                    ttl: 300,
                    priority: 0,
                },
                Record {
                    id: 106926659,
                    name: "www.borseth.ink".to_string(),
                    typ: RecordType::A,
                    content: "1.1.1.1".to_string(),
                    ttl: 300,
                    priority: 0,
                },
            ]
        );
    }

    #[test]
    fn edit_record() {
        let client = client();

        let m = mock("POST", "/dns/edit/borseth.ink/106926659")
            .match_body(Matcher::Json(
                body(json!({
                    "name": "www",
                    "type": "A",
                    "content": "1.1.1.2",
                    "ttl": "300",
                    "prio": "0"
                }))
                .unwrap(),
            ))
            .with_body(
                json!(
                    {
                        "status": "SUCCESS"
                    }
                )
                .to_string(),
            )
            .create();

        let res = client.edit_record(
            "borseth.ink",
            &Record {
                id: 106926659,
                name: "www".to_string(),
                typ: RecordType::A,
                content: "1.1.1.1".to_string(),
                ttl: 300,
                priority: 0,
            },
            "1.1.1.2".to_string(),
        );

        m.assert();
        assert!(res.is_ok());
    }

    #[test]
    fn create_record() {
        let client = client();

        let m = mock("POST", "/dns/create/borseth.ink")
            .match_body(Matcher::Json(
                body(json!({
                    "name": "www",
                    "type": "A",
                    "content": "1.1.1.1",
                    "ttl": "300",
                    "prio": "0"
                }))
                .unwrap(),
            ))
            .with_body(
                json!(
                    {
                        "status": "SUCCESS",
                        "id": "106926659"
                    }
                )
                .to_string(),
            )
            .create();

        let res = client.create_record(
            "borseth.ink",
            &Record {
                id: 0,
                name: "www".to_string(),
                typ: RecordType::A,
                content: "1.1.1.1".to_string(),
                ttl: 300,
                priority: 0,
            },
        );

        m.assert();
        assert!(res.is_ok());
    }
}
