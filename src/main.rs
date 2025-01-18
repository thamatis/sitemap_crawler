extern crate spider;

use std::sync::atomic::{AtomicUsize, Ordering};

use spider::tokio;
use spider::website::Website;

use html2md::parse_html;
use playwright::Playwright;
use reqwest::Error;
use std::env;

static GLOBAL_URL_COUNT: AtomicUsize = AtomicUsize::new(0);

async fn fetch_html(url: &str) -> Result<String, Error> {
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

async fn fetch_html_chrome(url: &str) -> Result<String, playwright::Error> {
    let playwright = Playwright::initialize().await?;
    playwright.prepare()?;
    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await?;
    let context = browser.context_builder().build().await?;
    let page = context.new_page().await?;

    page.goto_builder(url).goto().await?;

    let html_content = page.content().await?;
    Ok(html_content)
}

async fn convert_html_to_markdown(html: String) -> String {
    let _html: &str = html.as_str();
    parse_html(_html)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        println!("ไม่มีพารามิเตอร์");
        return;
    } else if args[1] != "http" && args[1] != "chrome" {
        println!("พารามิเตอร์ที่ป้อนไม่ใช่ http หรือ chrome");
        return;
    }

    let mut website: Website = Website::new("https://heygoody.com")
        .with_caching(true)
        .build()
        .unwrap();

    let start = std::time::Instant::now();

    let mut rx2 = website.subscribe(500).unwrap();

    let subscription = async move {
        while let Ok(res) = rx2.recv().await {
            if args[1] == "http" {
                match fetch_html(res.get_url()).await {
                    Ok(html) => {
                        // println!("HTML:\n{}", html)
                        let markdown = convert_html_to_markdown(html).await;
                        println!("{}", markdown);
                    }
                    Err(e) => eprintln!("Error at fetching URL: {}", e),
                }
            } else if args[1] == "chrome" {
                match fetch_html_chrome(res.get_url()).await {
                    Ok(html) => {
                        let markdown = convert_html_to_markdown(html).await;
                        println!("{}", markdown);
                    }
                    Err(e) => eprintln!("Error at fetching URL: {}", e),
                }
            }

            GLOBAL_URL_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    };

    let crawl = async move {
        website.crawl_raw().await;
        website.unsubscribe();
    };

    tokio::pin!(subscription);

    tokio::select! {
        _ = crawl => (),
        _ = subscription => (),
    };

    let duration = start.elapsed();

    println!(
        "Time elapsed in website.crawl() is: {:?} for total pages: {:?}",
        duration, GLOBAL_URL_COUNT
    )
}
