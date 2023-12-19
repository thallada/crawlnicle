use maud::{html, Markup, PreEscaped};

pub fn opml_import_form() -> Markup {
    html! {
        form
            id="opml-import-form"
            hx-post="/import/opml"
            hx-encoding="multipart/form-data"
            class="feed-form"
        {
            div class="form-grid" {
                label for="opml" { "OPML: " }
                input
                    type="file"
                    id="opml"
                    name="opml"
                    required="true"
                    accept="text/x-opml,application/xml,text/xml";
                button type="submit" { "Import Feeds" }
                progress id="opml-upload-progress" max="100" value="0" hidden="true" {}
            }
            script {
                (PreEscaped(r#"
                    htmx.on('#opml-import-form', 'htmx:xhr:progress', function (evt) {
                        htmx.find('#opml-upload-progress').setAttribute(
                            'value',
                            evt.detail.loaded / evt.detail.total * 100,
                        );
                    });
                "#))
            }
        }
    }
}
