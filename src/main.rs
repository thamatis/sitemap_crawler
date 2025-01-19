extern crate spider;

use spider::tokio;
use spider::website::Website;

use html2md::parse_html;
use reqwest::Error;
use std::env;
use futures::StreamExt;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;

async fn fetch_html(url: &str) -> Result<String, Error> {
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

async fn fetch_html_chrome(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (mut browser, mut handler) =
        Browser::launch(BrowserConfig::builder().with_head().build()?).await?;

    let handle = tokio::task::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(err) = event {
                eprintln!("Browser Event Error: {:?}", err);
            }
        }
    });

    let page = browser.new_page(url).await?;

    page.http_future(NavigateParams {
        url: url.to_string(),
        transition_type: None,
        frame_id: None,
        referrer: None,
        referrer_policy: None,
    })?
    .await?;

    if let Err(err) = page.wait_for_navigation().await {
        eprintln!("Navigation Error: {:?}", err);
    }

    let html_content = page.content().await?;

    browser.close().await?;
    handle.await?;

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


    let mut website: Website = Website::new("https://heygoody.com");

    website
        .configuration
        .with_respect_robots_txt(true)
        .with_user_agent(Some("SpiderBot"))
        .with_ignore_sitemap(true) // ignore running the sitemap on base crawl/scape methods. Remove or set to true to include the sitemap with the crawl.
        .with_sitemap(Some("/sitemap/sitemap-0.xml"));

    // crawl the sitemap first
    website.crawl_sitemap().await;
    // persist links to the next crawl
    website.persist_links();
    // crawl normal with links found in the sitemap extended.
    website.crawl().await;

    let links = website.get_all_links_visited().await;

    for link in links.iter() {
        if args[1] == "http" {
            match fetch_html(link).await {
                Ok(html) => {
                    // println!("HTML:\n{}", html)
                    let markdown = convert_html_to_markdown(html).await;
                    println!("{}", markdown);
                }
                Err(e) => eprintln!("Error at fetching URL: {}", e),
            }
        } else if args[1] == "chrome" {
            match fetch_html_chrome(link).await {
                Ok(html) => {
                    let markdown = convert_html_to_markdown(html).await;
                    println!("{}", markdown);
                }
                Err(e) => eprintln!("Error at fetching URL: {}", e),
            }
        }
    }
}
