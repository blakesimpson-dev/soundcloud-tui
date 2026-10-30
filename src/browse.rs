//! What the browse list shows: a stack of pages (a profile, a search, a playlist), each with
//! tabs, and the collections behind those tabs, which SoundCloud serves a page at a time.

use std::collections::HashMap;

use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppCommand;
use crate::config::SavedView;
use crate::soundcloud::{Entry, FeedItem, Playlist, SoundCloudClient, Track, User};

/// Items per page. A small first page shows the list sooner; the rest come in SoundCloud's
/// largest pages.
const FIRST_PAGE_SIZE: &str = "50";
const PAGE_SIZE: &str = "200";
const SEARCH_PAGE_SIZE: &str = "50";
/// `tracks?ids=` takes at most this many.
const TRACKS_PER_LOOKUP: usize = 50;
/// Playlist track lookups in flight at once.
const PARALLEL_LOOKUPS: usize = 4;

/// A list SoundCloud serves in pages.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Collection {
    Likes(u64),
    Uploads(u64),
    Reposts(u64),
    Playlists(u64),
    Albums(u64),
    SearchTracks(String),
    SearchUsers(String),
    SearchPlaylists(String),
    Playlist(u64),
}

impl Collection {
    pub fn label(&self) -> &'static str {
        match self {
            Collection::Likes(_) => "Likes",
            Collection::Uploads(_) => "Uploads",
            Collection::Reposts(_) => "Reposts",
            Collection::Playlists(_) => "Playlists",
            Collection::Albums(_) => "Albums",
            Collection::SearchTracks(_) => "Tracks",
            Collection::SearchUsers(_) => "Users",
            Collection::SearchPlaylists(_) => "Playlists",
            Collection::Playlist(_) => "Tracks",
        }
    }

    /// Profile lists and playlists are finite, so they keep loading in the background until
    /// complete; search results run on indefinitely, so they load a page at a time as the list
    /// is scrolled.
    pub fn loads_fully(&self) -> bool {
        !matches!(
            self,
            Collection::SearchTracks(_) | Collection::SearchUsers(_) | Collection::SearchPlaylists(_)
        )
    }
}

#[derive(Debug, Clone)]
pub enum Context {
    User(User),
    Search(String),
    Playlist(Playlist),
}

impl Context {
    pub fn title(&self) -> String {
        match self {
            Context::User(u) => format!("@{}", u.display_name()),
            Context::Search(q) => format!("Search: {q}"),
            Context::Playlist(p) => p.display_title().to_string(),
        }
    }

    pub fn tabs(&self) -> Vec<Collection> {
        match self {
            Context::User(u) => vec![
                Collection::Likes(u.id),
                Collection::Uploads(u.id),
                Collection::Reposts(u.id),
                Collection::Playlists(u.id),
                Collection::Albums(u.id),
            ],
            Context::Search(q) => vec![
                Collection::SearchTracks(q.clone()),
                Collection::SearchUsers(q.clone()),
                Collection::SearchPlaylists(q.clone()),
            ],
            Context::Playlist(p) => vec![Collection::Playlist(p.id)],
        }
    }

    pub fn saved(&self) -> SavedView {
        match self {
            Context::User(u) => SavedView::User { id: u.id },
            Context::Search(q) => SavedView::Search { query: q.clone() },
            Context::Playlist(p) => SavedView::Playlist { id: p.id },
        }
    }

    /// Fetches what a saved view needs to show again.
    pub async fn restore(client: &SoundCloudClient, view: &SavedView) -> Result<Context, String> {
        Ok(match view {
            SavedView::User { id } => Context::User(client.user(*id).await?),
            SavedView::Search { query } => Context::Search(query.clone()),
            SavedView::Playlist { id } => Context::Playlist(client.playlist(*id).await?),
        })
    }
}

/// A collection's entries so far.
#[derive(Default)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub next_href: Option<String>,
    pub complete: bool,
    pub loading: bool,
    pub error: Option<String>,
}

/// A page in the navigation stack.
pub struct Frame {
    pub context: Context,
    pub tab: usize,
    /// The selected row of each tab, kept while switching between them.
    pub selected: Vec<usize>,
}

#[derive(Default)]
pub struct Browser {
    pub stack: Vec<Frame>,
    /// Every collection loaded this session, so revisiting a tab or page is instant.
    pub listings: HashMap<Collection, Listing>,
}

impl Browser {
    pub fn frame(&self) -> Option<&Frame> {
        self.stack.last()
    }

    pub fn collection(&self) -> Option<Collection> {
        let frame = self.frame()?;
        frame.context.tabs().get(frame.tab).cloned()
    }

    pub fn listing(&self) -> Option<&Listing> {
        self.listings.get(&self.collection()?)
    }

    /// Starts loading the rest of `collection`, or its next page for one that loads on demand,
    /// unless it's already loading or complete.
    pub fn load_more(&mut self, collection: &Collection, client: &SoundCloudClient, tx: &UnboundedSender<AppCommand>) {
        let listing = self.listings.entry(collection.clone()).or_default();
        if listing.loading || listing.complete {
            return;
        }
        listing.loading = true;
        listing.error = None;
        spawn_load(client.clone(), tx.clone(), collection.clone(), listing.next_href.clone());
    }

    pub fn add_page(&mut self, collection: Collection, page: Page) {
        let listing = self.listings.entry(collection).or_default();
        listing.entries.extend(page.entries);
        listing.next_href = page.next_href;
        listing.complete = page.complete;
        listing.loading = page.more;
    }

    pub fn load_failed(&mut self, collection: Collection, error: String) {
        let listing = self.listings.entry(collection).or_default();
        listing.loading = false;
        listing.error = Some(error);
    }
}

/// Entries loaded for a collection, sent back to the UI as they arrive.
#[derive(Debug)]
pub struct Page {
    pub entries: Vec<Entry>,
    pub next_href: Option<String>,
    pub complete: bool,
    /// Whether the loader is fetching another page after this one.
    pub more: bool,
}

fn spawn_load(client: SoundCloudClient, tx: UnboundedSender<AppCommand>, collection: Collection, mut next: Option<String>) {
    tokio::spawn(async move {
        if let Collection::Playlist(id) = collection {
            return load_playlist(&client, &tx, id).await;
        }
        loop {
            let (entries, next_href) = match fetch_page(&client, &collection, next.as_deref()).await {
                Ok(page) => page,
                Err(error) => {
                    let _ = tx.send(AppCommand::LoadFailed { collection, error });
                    return;
                }
            };
            // A repeated next_href would loop forever; treat it as the end.
            let complete = next_href.is_none() || next_href == next;
            // An empty page gives the list nothing to scroll toward, so fetch on regardless.
            let more = !complete && (collection.loads_fully() || entries.is_empty());
            next = next_href.clone();
            let page = Page { entries, next_href, complete, more };
            if tx.send(AppCommand::Page { collection: collection.clone(), page }).is_err() || !more {
                return;
            }
        }
    });
}

async fn fetch_page(
    client: &SoundCloudClient,
    collection: &Collection,
    next: Option<&str>,
) -> Result<(Vec<Entry>, Option<String>), String> {
    let (endpoint, params): (String, Vec<(&str, &str)>) = match (next, collection) {
        (Some(next), _) if collection.loads_fully() => (with_limit(next, PAGE_SIZE), vec![]),
        (Some(next), _) => (next.to_string(), vec![]),
        (None, Collection::Likes(id)) => (format!("users/{id}/likes"), vec![("limit", FIRST_PAGE_SIZE)]),
        (None, Collection::Uploads(id)) => (format!("users/{id}/tracks"), vec![("limit", FIRST_PAGE_SIZE)]),
        (None, Collection::Reposts(id)) => {
            (format!("stream/users/{id}/reposts"), vec![("limit", FIRST_PAGE_SIZE)])
        }
        (None, Collection::Playlists(id)) => {
            (format!("users/{id}/playlists_without_albums"), vec![("limit", FIRST_PAGE_SIZE)])
        }
        (None, Collection::Albums(id)) => (format!("users/{id}/albums"), vec![("limit", FIRST_PAGE_SIZE)]),
        (None, Collection::SearchTracks(q)) => ("search/tracks".into(), vec![("q", q), ("limit", SEARCH_PAGE_SIZE)]),
        (None, Collection::SearchUsers(q)) => ("search/users".into(), vec![("q", q), ("limit", SEARCH_PAGE_SIZE)]),
        (None, Collection::SearchPlaylists(q)) => {
            ("search/playlists".into(), vec![("q", q), ("limit", SEARCH_PAGE_SIZE)])
        }
        (None, Collection::Playlist(_)) => unreachable!("playlists load through load_playlist"),
    };

    match collection {
        Collection::Likes(_) | Collection::Reposts(_) => {
            let page = client.page::<FeedItem>(&endpoint, &params).await?;
            Ok((page.collection.into_iter().filter_map(FeedItem::into_entry).collect(), page.next_href))
        }
        Collection::Uploads(_) | Collection::SearchTracks(_) => {
            let page = client.page::<Track>(&endpoint, &params).await?;
            Ok((page.collection.into_iter().map(Entry::Track).collect(), page.next_href))
        }
        Collection::Playlists(_) | Collection::Albums(_) | Collection::SearchPlaylists(_) => {
            let page = client.page::<Playlist>(&endpoint, &params).await?;
            Ok((page.collection.into_iter().map(Entry::Playlist).collect(), page.next_href))
        }
        Collection::SearchUsers(_) => {
            let page = client.page::<User>(&endpoint, &params).await?;
            Ok((page.collection.into_iter().map(Entry::User).collect(), page.next_href))
        }
        Collection::Playlist(_) => unreachable!("playlists load through load_playlist"),
    }
}

/// `href` with its `limit` replaced: a `next_href` repeats the limit of the request it came from.
fn with_limit(href: &str, limit: &str) -> String {
    let Ok(mut url) = url::Url::parse(href) else { return href.to_string() };
    let pairs: Vec<(String, String)> = url.query_pairs().into_owned().filter(|(k, _)| k != "limit").collect();
    url.query_pairs_mut().clear().extend_pairs(pairs).append_pair("limit", limit);
    url.to_string()
}

/// A playlist arrives whole, but only its first few tracks are complete; the rest are ids,
/// looked up in batches and sent on in playlist order.
async fn load_playlist(client: &SoundCloudClient, tx: &UnboundedSender<AppCommand>, id: u64) {
    let collection = Collection::Playlist(id);
    let fail = |error| {
        let _ = tx.send(AppCommand::LoadFailed { collection: collection.clone(), error });
    };
    let tracks = match client.playlist(id).await {
        Ok(playlist) => playlist.tracks.unwrap_or_default(),
        Err(e) => return fail(e),
    };

    // Split into runs that each end after a batch of stubs, look the batches up a few at a
    // time, and send each run on as soon as it and every run before it are ready.
    let mut runs: Vec<Vec<Track>> = Vec::new();
    let mut stubs_in_run = 0;
    for track in tracks {
        if track.is_stub() {
            if stubs_in_run == TRACKS_PER_LOOKUP {
                stubs_in_run = 0;
                runs.push(Vec::new());
            }
            stubs_in_run += 1;
        }
        match runs.last_mut() {
            Some(run) => run.push(track),
            None => runs.push(vec![track]),
        }
    }

    // Keep a few lookups in flight, started and sent in playlist order.
    let lookup = |run: Vec<Track>| {
        let client = client.clone();
        tokio::spawn(async move {
            let stubs: Vec<u64> = run.iter().filter(|t| t.is_stub()).map(|t| t.id).collect();
            if stubs.is_empty() {
                return Ok(run);
            }
            let mut found: HashMap<u64, Track> =
                client.tracks_by_ids(&stubs).await?.into_iter().map(|t| (t.id, t)).collect();
            Ok::<_, String>(run.into_iter().filter_map(|t| if t.is_stub() { found.remove(&t.id) } else { Some(t) }).collect())
        })
    };
    let mut runs = runs.into_iter();
    let mut in_flight: std::collections::VecDeque<_> = runs.by_ref().take(PARALLEL_LOOKUPS).map(lookup).collect();
    if in_flight.is_empty() {
        let page = Page { entries: Vec::new(), next_href: None, complete: true, more: false };
        let _ = tx.send(AppCommand::Page { collection: collection.clone(), page });
        return;
    }
    while let Some(next) = in_flight.pop_front() {
        in_flight.extend(runs.next().map(lookup));
        let run: Vec<Track> = match next.await {
            Ok(Ok(run)) => run,
            Ok(Err(e)) => return fail(e),
            Err(e) => return fail(e.to_string()),
        };
        let entries = run.into_iter().map(Entry::Track).collect();
        let complete = in_flight.is_empty();
        let page = Page { entries, next_href: None, complete, more: !complete };
        if tx.send(AppCommand::Page { collection: collection.clone(), page }).is_err() || complete {
            return;
        }
    }
}
