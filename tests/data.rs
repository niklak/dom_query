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
        <head><title>Test</title></head>
        <body>
            <!--Ancestors-->
           <div id="great-ancestor">
               <div id="grand-parent">
                   <div id="parent">
                       <div id="first-child" class="child">Child</div>
                       <div id="second-child" class="child">Child</div>
                   </div>
               </div>
               <div id="grand-parent-sibling"></div>
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

pub static ATTRS_CONTENTS: &str = r#"<!DOCTYPE html>
    <html>
        <head></head>
        <body>
            <div id="main">
                <div><font face="Times" size="10" color="green">Lorem</font></div>
                <div><font face="Arial" size="8" color="red">ipsum dolor</font></div>
                <div><font face="Courier" size="5" color="red">sit amet</font></div>
            </div>
        </body>
    </html>"#;

pub static DMC_CONTENTS: &str = r#"<!DOCTYPE html>
<html>
    <head></head>
    <body>
        <div id="main">
            <div>
                <p>Listen up y'all, it's time to get down</p>
                <p>'Bout that <b>normalized_char_count</b> in this town</p>
                <p>Traversing nodes with style and grace</p>
                <p>Counting chars at a steady pace</p>
            </div>

            <div>
                <p>No split whitespace, that's old school</p>
                <p>Direct counting's our golden rule</p>
                <p>Skip them nodes that ain't text or element</p>
                <p>That's how we keep our code development!</p>
            </div>
        </div>
    </body>
</html>"#;
