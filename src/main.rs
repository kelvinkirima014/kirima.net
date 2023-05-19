use std::convert::{ Infallible, From };
use std::io::{Error, Read, self};
use std::fs::{self, File};
use std::path::PathBuf;
use std::io::Write;
use handlebars::Handlebars;
use hyper::service::{service_fn, make_service_fn};
use hyper::{ Body, Request, Response, Server };
use serde_json::json;
use pulldown_cmark::{html, Options, Parser};

pub struct Posts {
    post_path: PathBuf,
}

impl Posts {
    fn new(post_path: PathBuf) -> Self {
        Posts {
            post_path,
        }
    }

    fn fetch_posts(&self) -> Result<Vec<(PathBuf, String)>, Error>{

        let mut posts = vec![];

        let entries = fs::read_dir(&self.post_path)?;
            //iterate over the contents of the directory
            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                //check if the entry is a file and has .md extension
                if entry.file_type()?.is_file() && path.extension()
                    .and_then(|e| e.to_str()) == Some("md") {
                        let mut file = File::open(&path)?;
                        let mut contents = String::new();
                        file.read_to_string(&mut contents)?;
                        posts.push((path, contents));
                    }
            }

            Ok(posts)

    }
}


fn md_to_html(markdown: &str) -> String {
    // Set up the parser with default options
    let parser = Parser::new_ext(markdown, Options::empty());

    // Convert the markdown to HTML
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}

#[tokio::main]
async fn main () -> Result<(), Error> {
    let mut handlebars = Handlebars::new();
    //let custom_template = "<html><body>{{{content}}}</body></html>";
    handlebars.register_template_file("post_template", "templates/posts.hbs")
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let post_path = PathBuf::from("./markdown");

    let posts: Posts = Posts::new(post_path);

    let fetch_posts = posts.fetch_posts()?;

    let mut index_file = File::create("index.html")?;

    for (path, contents) in fetch_posts {
        let _post_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("no title found");

        //let post_title_link = format!("<a href=\"{}\">{}</a>", path.display(), post_title);

        let html = handlebars.render("post_template", &json!({
            //"title": post_title,
            "content": md_to_html(&contents),
        }));

        index_file.write_all(html.expect("err").as_bytes())?;

    }
   
    let addr = ([127, 0, 0, 1], 3000).into();

    let make_serve = make_service_fn(|_| async {
        let content =  fs::read("index.html").unwrap();
        Ok::<_, Infallible>(service_fn(move |_: Request<Body>| {
            let response = Response::new(Body::from(content.clone()));
            async move {
                Ok::<_, Infallible>(response)
            }
        }))
    });

    let server = Server::bind(&addr).serve(make_serve);

    println!("Listening on http://{}", addr);
    
    server.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
  
}