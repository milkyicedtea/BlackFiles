use super::*;

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct BookmarksResponse {
    pub bookmarks: EmptyResponse,
}

#[get("/getBookmarks")]
pub(crate) fn get_bookmarks(_user: SubsonicUser) -> Json<serde_json::Value> {
    ok_resp(BookmarksResponse {
        bookmarks: EmptyResponse {},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_bookmarks_returns_an_empty_bookmarks_container() {
        let response = get_bookmarks(SubsonicUser {
            id: Uuid::nil(),
            username: "test".into(),
        })
        .into_inner();
        let body = &response["subsonic-response"];

        assert_eq!(body["status"], "ok");
        assert_eq!(body["bookmarks"], serde_json::json!({}));
    }
}
