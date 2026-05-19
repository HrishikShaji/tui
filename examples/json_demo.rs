use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Paragraph {
    name: String,
}

#[derive(Serialize, Deserialize)]
struct Article {
    article: String,
    author: String,
    paragraph: Vec<Paragraph>,
}

fn main() {
    // Create JSON
    let new_article = Article {
        article: String::from("How to write rust"),
        author: String::from("shaji"),
        paragraph: vec![
            Paragraph {
                name: String::from("macro"),
            },
            Paragraph {
                name: String::from("micro"),
            },
        ],
    };
    let new_json = serde_json::to_string(&new_article).unwrap();
    println!("The created json is {:?}", new_json);

    // Parse JSON
    let json = r#"
    {
    "article":"how to learn rust",
    "author":"hrishik",
    "paragraph":[
    {
    "name":"one"
    },
    {
    "name":"two"
    }
    ]
    }
    "#;
    let parsed: Article = serde_json::from_str(json).unwrap();
    println!(
        "\n\n The name of first paragraph is :{}",
        parsed.paragraph[0].name
    );
}
