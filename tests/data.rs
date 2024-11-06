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
                       <div id="first-child">Child</div>
                       <div id="second-child">Child</div>
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

pub static HEADING_CONTENTS: &str = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
            <div class="content heading">
                <h1>Test Page</h1>
            </div>
            <div class="content text-content">
                <p>This is a test page contents.</p>
            </div
        </body>
    </html>"#;

pub static REPLACEMENT_CONTENTS: &str = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div id="main">
                <p id="before-origin"></p>
                <p id="origin"><span id="inline">Something</span></p>
                <p id="after-origin"><span>About</span><span>Me</span></p>
            </div>
        </body>
    </html>"#;

pub static REPLACEMENT_SEL_CONTENTS: &str = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div class="ad-content">
                <p><span></span></p>
                <p><span></span></p>
            </div>
            <span class="source">example</span>
        </body>
    </html>"#;

pub static EMPTY_BLOCKS_CONTENTS: &str = r#"<!DOCTYPE html>
    <html>
        <head></head>
        <body>
            <div id="main">
                <div></div>
                <div></div>
            </div>
        </body>
    </html>"#;
