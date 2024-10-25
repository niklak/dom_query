#![allow(dead_code)]
use dom_query::Document;

pub fn doc() -> Document {
    include_str!("../test-pages/page.html").into()
}

pub fn doc_wiki() -> Document {
    include_str!("../test-pages/rustwiki.html").into()
}

pub fn doc_with_siblings() -> Document {
    include_str!("../test-pages/tests_with_siblings.html").into()
}

pub static ANCESTORS_CONTENTS: &str = r#"<!DOCTYPE html>
    <html>
        <head>Test</head>
        <body>
           <div id="great-ancestor">
               <div id="grand-parent">
                   <div id="parent">
                       <div id="child">Child</div>
                   </div>
               </div>
           </div>
        </body>
    </html>"#;

pub static LIST_CONTENTS: &str = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div>
                <ul class="list">
                    <li>1</li><li>2</li><li>3</li>
                </ul>
                <ul class="list">
                    <li>4</li><li>5</li><li>6</li>
                </ul>
            <div>
        </body>
    </html>"#;
