use async_std::task;
use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, TagToken, Token, TokenSink, TokenSinkResult, Tokenizer,
    TokenizerOpts,
};
use std::borrow::Borrow;
use url::{ParseError, Url};

// Struct to store links
#[derive(Default, Debug)]
struct LinkQueue {
    links: Vec<String>,
}

// Implement TokenSink for LinkQueue
impl TokenSink for &mut LinkQueue {
    type Handle = ();

    fn process_token(&mut self, token: Token, _line_number: u64) -> TokenSinkResult<Self::Handle> {
        match token {
            TagToken(tag) => {
                if tag.kind == TagKind::StartTag && tag.name.as_ref() == "a" {
                    for attribute in tag.attrs.iter() {
                        if attribute.name.local.as_ref() == "href" {
                            let url_str: &[u8] = attribute.value.borrow();
                            self.links
                                .push(String::from_utf8_lossy(url_str).into_owned());
                        }
                    }
                }
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

// Function to extract links from a page
fn get_links(url: &Url, page: String) -> Vec<Url> {
    let mut domain_url = url.clone();
    domain_url.set_path("");
    domain_url.set_query(None);

    let mut queue = LinkQueue::default();
    let mut tokenizer = Tokenizer::new(&mut queue, TokenizerOpts::default());
    let mut buffer = BufferQueue::new();
    buffer.push_back(page.into());
    let _ = tokenizer.feed(&mut buffer);

    queue
        .links
        .iter()
        .map(|link| match Url::parse(link) {
            Err(ParseError::RelativeUrlWithoutBase) => domain_url.join(link).unwrap(),
            Err(_) => panic!("Malformed link found: {}", link),
            Ok(url) => url,
        })
        .collect()
}

// Type alias for result
type CrawlResult = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

// Boxed future type
type BoxFuture = std::pin::Pin<Box<dyn std::future::Future<Output = CrawlResult> + Send>>;

// Async crawl function
async fn crawl(pages: Vec<Url>, current: u8, max: u8) -> CrawlResult {
    if current > max {
        println!("Reached Max Depth");
        return Ok(());
    }

    println!("Current Depth: {}, Max Depth: {}", current, max);
    println!("Crawling: {:?}", pages);

    let mut tasks = vec![];

    for url in pages {
        let task = task::spawn(async move {
            println!("Getting: {}", url);

            let mut res = surf::get(&url).await?;
            let body = res.body_string().await?;

            let links = get_links(&url, body);

            println!("Following: {:?}", links);

            box_crawl(links, current + 1, max).await
        });

        tasks.push(task);
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}

// Box crawl helper
fn box_crawl(pages: Vec<Url>, current: u8, max: u8) -> BoxFuture {
    Box::pin(crawl(pages, current, max))
}

// Entry point
fn main() -> CrawlResult {
    task::block_on(async {
        box_crawl(vec![Url::parse("https://www.meesho.com/").unwrap()], 1, 2).await
    })
}
