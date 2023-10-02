pub struct Etherscan {
    api_key: String,
}

impl Etherscan {
    pub fn new(api_key: String) -> Self {
        Etherscan { api_key }
    }

    pub fn tx_list_internal(&self, address: String) -> serde_json::Value {
        // https://api.etherscan.io/api
        //    ?module=account
        //    &action=txlistinternal
        //    &txhash=0x40eb908387324f2b575b4879cd9d7188f69c8fc9d87c901b9e2daaea4b442170
        //    &apikey=YourApiKeyToken

        let url = format!(
            "https://api.etherscan.io/api?module=account&action=txlistinternal&txhash={}&apikey={}",
            address, self.api_key
        );
        log::info!("{}", url);
        ureq::get(&url).call().unwrap().into_json().unwrap()
    }
}
