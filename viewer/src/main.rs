use gloo_net::http::Request;
use serde::Deserialize;
use std::fmt;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

// 도시 검색 결과 타입
#[derive(Clone, PartialEq, Deserialize)]
struct City {
    id: String,
    name: String,
    asciiname: String,
    country: String,
    population: i64,
    timezone: String,
}

// 메인 컴포넌트 상태
struct CitySearch {
    query: String,
    results: Vec<City>,
    loading: bool,
}

// 컴포넌트 메시지
enum Msg {
    QueryInput(String),
    Search,
    ReceiveResults(Vec<City>),
    Error(String),
}

impl Component for CitySearch {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            loading: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::QueryInput(query) => {
                self.query = query;
                true
            }
            Msg::Search => {
                self.loading = true;
                let query = self.query.clone();

                // API 호출을 위한 클로저
                let link = ctx.link().clone();
                spawn_local(async move {
                    match search_cities(&query).await {
                        Ok(results) => link.send_message(Msg::ReceiveResults(results)),
                        Err(e) => link.send_message(Msg::Error(e.to_string())),
                    }
                });
                true
            }
            Msg::ReceiveResults(results) => {
                self.results = results;
                self.loading = false;
                true
            }
            Msg::Error(_) => {
                self.loading = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="container mx-auto p-4">
                <h1 class="text-3xl font-bold mb-4">{"City Search"}</h1>

                // 검색 폼
                <div class="mb-4 flex gap-2">
                    <input
                        type="text"
                        value={self.query.clone()}
                        onkeypress={link.batch_callback(|e: KeyboardEvent| {
                            if e.key() == "Enter" { Some(Msg::Search) } else { None }
                        })}
                        oninput={link.callback(|e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            Msg::QueryInput(input.value())
                        })}
                        class="flex-1 p-2 border rounded"
                        placeholder="도시 이름을 입력하세요..."
                    />
                    <button
                        onclick={link.callback(|_| Msg::Search)}
                        class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
                        disabled={self.loading}
                    >
                        {"검색"}
                    </button>
                </div>

                // 로딩 표시
                if self.loading {
                    <div class="text-center py-4">
                        {"검색 중..."}
                    </div>
                }

                // 결과 목록
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    {for self.results.iter().map(|city| {
                        html! {
                            <div class="p-4 border rounded shadow">
                                <h3 class="text-xl font-semibold">{&city.name}</h3>
                                <p class="text-gray-600">{&city.country}</p>
                                <div class="mt-2">
                                    <p class="text-sm">{"인구: "}{format_population(city.population)}</p>
                                    <p class="text-sm">{"시간대: "}{&city.timezone}</p>
                                </div>
                            </div>
                        }
                    })}
                </div>
            </div>
        }
    }
}

#[derive(Debug)]
enum SearchError {
    Network(gloo_net::Error),
    Parsing(String), // 일반적인 문자열 기반 파싱 에러
    Other(String),
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchError::Network(err) => write!(f, "Network error: {}", err),
            SearchError::Parsing(msg) => write!(f, "Parsing error: {}", msg),
            SearchError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl From<gloo_net::Error> for SearchError {
    fn from(error: gloo_net::Error) -> Self {
        SearchError::Network(error)
    }
}

#[derive(Clone, Deserialize)]
struct MeiliSearchResponse {
    hits: Vec<City>,
    // 다른 메타데이터 필드들 (필요하다면)
    // offset: usize,
    // limit: usize,
    // total: usize
}

async fn search_cities(query: &str) -> Result<Vec<City>, SearchError> {
    let url = "http://localhost:7700/indexes/cities/search".to_string();

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer eSampleMasterKey")
        .json(&serde_json::json!({
            "q": query,
            "limit": 20
        }))
        .map_err(|e| SearchError::Other(e.to_string()))?
        .send()
        .await?;

    let meilisearch_response: MeiliSearchResponse = response
        .json()
        .await
        .map_err(|e| SearchError::Parsing(e.to_string()))?;

    Ok(meilisearch_response.hits)
}

// 인구수 포맷팅 함수
fn format_population(pop: i64) -> String {
    if pop >= 1_000_000 {
        format!("{:.1}M", pop as f64 / 1_000_000.0)
    } else if pop >= 1_000 {
        format!("{:.1}K", pop as f64 / 1_000.0)
    } else {
        pop.to_string()
    }
}

fn console_log_init() {
    web_sys::console::log_1(&"Application started!".into());
}

fn main() {
    console_log_init(); // 콘솔 로깅 초기화
    yew::Renderer::<CitySearch>::new().render();
}
