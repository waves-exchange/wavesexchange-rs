use crate::{ApiResult, BaseApi, HttpClient};

#[derive(Clone, Debug)]
pub struct Levex;

impl BaseApi for Levex {}

impl HttpClient<Levex> {
    pub async fn leveraged_tokens_summary(&self) -> ApiResult<dto::SummaryResponse> {
        self.create_req_handler(self.http_get("summary"), "levex::leveraged_tokens_summary")
            .execute()
            .await
    }
}

#[allow(dead_code)]
pub mod dto {
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SummaryResponse {
        pub pairs: Vec<Pair>,
        pub config: Config,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Pair {
        pub pair_id: String,
        pub bear_id: String,
        pub bull_id: String,
        pub pool_id: String,
        pub leverage_factor: u64,
        pub max_issue_bull: String,
        pub max_issue_bear: String,
        pub price_change: [PricePair; 2],
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PricePair {
        pub bear: [String; 2],
        pub bull: [String; 2],
        pub price_index: u64,
        pub timestamp: u64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Config {
        pub usdn_pacemaker_fee: u64,
        pub waves_pacemaker_fee: u64,
        pub issue_percentile: u64,
        pub redeem_percentile: u64,
        pub min_issue: u64,
        pub min_pool: u64,
        pub min_redeem: u64,
    }
}

#[cfg(test)]
mod tests {
    use super::dto::*;

    #[test]
    fn leveraged_tokens_summary_response_parse() {
        let json = r#"
            {
              "pairs": [
                {
                  "pairId": "3P3b9ZcfQmAyE9MVoRKE5tfRJSHR4BDXMEo",
                  "bearId": "45WTLz6e3Ek8Ffe7QHMkQ2TwfozfWsrodTHMtPyTMNtt",
                  "bullId": "HiiB3SSS1c89J5qQ6RLTUx4qgszLMQS2WRC3wfGfaCF8",
                  "poolId": "Ereo35igXbBwcwe9mxxjEUFgMk7FGQUioSUELMwZWYT8",
                  "leverageFactor": 3,
                  "maxIssueBull": "12835022",
                  "maxIssueBear": "212798970915",
                  "priceChange": [
                    {
                      "bear": [
                        "47786294211",
                        "179825313521868527"
                      ],
                      "bull": [
                        "186103583782",
                        "439684589734"
                      ],
                      "priceIndex": 92675,
                      "timestamp": 1630642643964
                    },
                    {
                      "bear": [
                        "53465296877",
                        "170678993628027315"
                      ],
                      "bull": [
                        "182583051642",
                        "444784589734"
                      ],
                      "priceIndex": 91272,
                      "timestamp": 1630556235570
                    }
                  ]
                },
                {
                  "pairId": "3P9ZegsKUtsEpdRPNVrMH7nHEEqY5MrmjDp",
                  "bearId": "DRVGiwqmsZpFzaMoAFQXjBNXT4PFepgPvnJ5sGUrhXQt",
                  "bullId": "8b53M5vTk8wRBRuJ27ebTvTeGfbjpLZuoZQ7hkFjHsu4",
                  "poolId": "EEEyg2QxvZj5KmWjhEXBhVAofojocZbzR2Lvm7Q3TAps",
                  "leverageFactor": 3,
                  "maxIssueBull": "529364019604",
                  "maxIssueBear": "739802174108",
                  "priceChange": [
                    {
                      "bear": [
                        "126027632966",
                        "440604382440797"
                      ],
                      "bull": [
                        "126026536502",
                        "550205810791"
                      ],
                      "priceIndex": 94244,
                      "timestamp": 1630642643964
                    },
                    {
                      "bear": [
                        "130517671457",
                        "452884038431273"
                      ],
                      "bull": [
                        "130513080197",
                        "563795850564"
                      ],
                      "priceIndex": 92841,
                      "timestamp": 1630556235570
                    }
                  ]
                }
              ],
              "config": {
                "usdnPacemakerFee": 100000,
                "wavesPacemakerFee": 500000,
                "issuePercentile": 100,
                "redeemPercentile": 100,
                "minIssue": 10000000,
                "minPool": 10000000,
                "minRedeem": 10000000
              }
            }
        "#;

        let r: SummaryResponse = serde_json::from_str(json).unwrap();

        assert_eq!(r.pairs.len(), 2);
        assert_eq!(r.pairs[0].pair_id, "3P3b9ZcfQmAyE9MVoRKE5tfRJSHR4BDXMEo");
        assert_eq!(
            r.pairs[0].bear_id,
            "45WTLz6e3Ek8Ffe7QHMkQ2TwfozfWsrodTHMtPyTMNtt"
        );
        assert_eq!(
            r.pairs[0].bull_id,
            "HiiB3SSS1c89J5qQ6RLTUx4qgszLMQS2WRC3wfGfaCF8"
        );
        assert_eq!(
            r.pairs[0].pool_id,
            "Ereo35igXbBwcwe9mxxjEUFgMk7FGQUioSUELMwZWYT8"
        );
        assert_eq!(r.pairs[0].leverage_factor, 3);
        assert_eq!(r.pairs[0].max_issue_bull, "12835022");
        assert_eq!(r.pairs[0].max_issue_bear, "212798970915");
        assert_eq!(
            r.pairs[0].price_change[0].bear,
            ["47786294211", "179825313521868527"]
        );
        assert_eq!(
            r.pairs[0].price_change[0].bull,
            ["186103583782", "439684589734"]
        );
        assert_eq!(r.pairs[0].price_change[0].price_index, 92675);
        assert_eq!(r.pairs[0].price_change[0].timestamp, 1630642643964);
        assert_eq!(
            r.pairs[0].price_change[1].bear,
            ["53465296877", "170678993628027315"]
        );
        assert_eq!(
            r.pairs[0].price_change[1].bull,
            ["182583051642", "444784589734"]
        );
        assert_eq!(r.pairs[0].price_change[1].price_index, 91272);
        assert_eq!(r.pairs[0].price_change[1].timestamp, 1630556235570);
        assert_eq!(r.config.usdn_pacemaker_fee, 100000);
        assert_eq!(r.config.waves_pacemaker_fee, 500000);
        assert_eq!(r.config.issue_percentile, 100);
        assert_eq!(r.config.redeem_percentile, 100);
        assert_eq!(r.config.min_issue, 10000000);
        assert_eq!(r.config.min_pool, 10000000);
        assert_eq!(r.config.min_redeem, 10000000);
    }
}
