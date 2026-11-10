//! Reading a list GitHub splits into pages of 100, in its two ways: pages
//! linked by the `Link` header (`rel="next"`), and numbered pages
//! (`page=1, 2, …`) whose answers carry the list's `total_count`.

use serde::de::DeserializeOwned;

use crate::github_client::{JSON_MEDIA_TYPE, PAGE_SIZE};
use crate::{GitHubClient, GitHubError};

/// A numbered page's answer: its items and the total the list holds.
pub(crate) trait NumberedPage {
    /// How many items this page holds.
    fn item_count(&self) -> usize;
    /// How many items the whole list holds.
    fn total_count(&self) -> usize;
}

impl GitHubClient {
    /// Reads the JSON list at `first_url` (asked with `query`, the page
    /// size included) page by page, following each answer's `rel="next"`
    /// link (which already carries the query), at most `max_pages` pages,
    /// handing each page's items to `on_page`; an error from `on_page` stops
    /// the reading. Returns whether the pages ran out before `max_pages`:
    /// every item asked for was read.
    pub(crate) fn linked_pages<T: DeserializeOwned>(
        &self,
        first_url: &str,
        query: &[(&str, &str)],
        max_pages: usize,
        on_page: &mut dyn FnMut(Vec<T>) -> Result<(), GitHubError>,
    ) -> Result<bool, GitHubError> {
        let mut next = Some((first_url.to_owned(), query.to_vec()));
        for _ in 0..max_pages {
            let Some((url, query)) = next.take() else {
                break;
            };
            let response = self.get(&url, &query, None, JSON_MEDIA_TYPE, &mut |_| {})?;
            on_page(serde_json::from_str(&response.body)?)?;
            // The next page's URL already carries its query.
            next = response.next_page.map(|url| (url, Vec::new()));
        }
        Ok(next.is_none())
    }

    /// Reads the list at `url` (asked with `query`, 100 per page) as
    /// numbered pages from 1, at most `max_pages`, handing each answer to
    /// `on_page`; stops at an empty page or once the items read reach the
    /// answer's total.
    pub(crate) fn numbered_pages<P: DeserializeOwned + NumberedPage>(
        &self,
        url: &str,
        query: &[(&str, &str)],
        max_pages: usize,
        on_page: &mut dyn FnMut(P),
    ) -> Result<(), GitHubError> {
        let mut read = 0;
        for page in 1..=max_pages {
            // This page's query: the list's, the page size, the page number.
            let page_text = page.to_string();
            let mut page_query = query.to_vec();
            page_query.push(("per_page", PAGE_SIZE));
            page_query.push(("page", &page_text));
            let answer: P = self.get_json(url, &page_query)?;

            // Hand it on, then stop once nothing more is left to read.
            let count = answer.item_count();
            let total = answer.total_count();
            read += count;
            on_page(answer);
            if count == 0 || read >= total {
                break;
            }
        }
        Ok(())
    }
}
