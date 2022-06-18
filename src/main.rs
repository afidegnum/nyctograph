use rexie::*;
use serde::Deserialize;
use serde::Serialize;
use std::rc::Rc;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::{use_transition, Suspense};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
/*
-  List elements
  Add/Remove Elements
ElementFormatting
Select

[
{ "uuid" : "adsa234", "tag" : "h1", "text" : "This is the heading", "order" : 0 },
{ "uuid" : "adsa234", "tag" : "p", "text" : "Welcome to the page", "order" : 1 },
{ "uuid" : "adsa234", "tag" : "p", "text" : "another line", "order" : 2 },
{ "uuid" : "adsa234", "tag" : "ul", "text" : ["item1", "item2", "item3"], "order" : 3 },
{ "uuid" : "adsa234", "tag" : "p", "text" : "another line", "order" : 4 },
{ "uuid" : "adsa234", "tag" : "table", "text" : ["col1", "col2"], ["row1-col1", "row2-col3"], "order" : 3 },
]

workflow:
load-json
single-node-from-json
node-list-fron-json
add-to-node-list
remove-to-node-list

*/

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomRecord {
    pub id: u32,
    pub uuid: Uuid,
    pub tag: String,
    pub text: String,
    pub order: u16,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonFrontDom {
    pub uuid: Uuid,
    pub tag: String,
    pub text: String,
    pub order: u16,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonDom<'a> {
    pub uuid: Uuid,
    pub tag: &'a str,
    pub text: &'a str,
    pub order: u16,
}
pub type DomNodes = Vec<JsonFrontDom>;

/// Creates a database
async fn dom_db() -> Rexie {
    assert!(Rexie::delete("test").await.is_ok());

    let rexie = Rexie::builder("domdb")
        .version(1)
        .add_object_store(
            ObjectStore::new("domnodes")
                .key_path("id")
                .auto_increment(true)
                .add_index(Index::new("uuid", "uuid").unique(true)),
        )
        .build()
        .await;
    assert!(rexie.is_ok());
    rexie.unwrap()
}

async fn insert_node(rexie: &Rexie, tag: &str, text: &str, order: u16) -> Result<u32> {
    let transaction = rexie.transaction(&["domnodes"], TransactionMode::ReadWrite);
    assert!(transaction.is_ok());
    let transaction = transaction.unwrap();

    let local_nodes = transaction.store("domnodes");
    assert!(local_nodes.is_ok());
    let local_nodes = local_nodes.unwrap();

    let uuid = Uuid::new_v4();

    let local_node = JsonDom {
        uuid,
        tag,
        text,
        order,
    };

    let local_node = serde_wasm_bindgen::to_value(&local_node).unwrap();
    let local_node_id = local_nodes.add(&local_node, None).await?;

    transaction.commit().await?;
    Ok(num_traits::cast(local_node_id.as_f64().unwrap()).unwrap())
    // modify to return last inserted ID
}

async fn fetch_json_nodes(rexie: &Rexie, direction: Option<Direction>) -> Result<Vec<DomRecord>> {
    let transaction = rexie.transaction(&["domnodes"], TransactionMode::ReadOnly);
    assert!(transaction.is_ok());
    let transaction = transaction.unwrap();

    let node_records = transaction.store("domnodes");
    assert!(node_records.is_ok());
    let node_records = node_records.unwrap();

    let node_records: Vec<JsValue> = node_records
        .get_all(None, None, None, direction)
        .await?
        .into_iter()
        .map(|pair| pair.1)
        .collect();

    let node_records: Vec<DomRecord> = node_records
        .into_iter()
        .map(|node_record| serde_wasm_bindgen::from_value(node_record).unwrap())
        .collect();

    Ok(node_records)
}

async fn count_node_records(rexie: &Rexie, key_range: Option<&KeyRange>) -> Result<u32> {
    let transaction = rexie.transaction(&["domnodes"], TransactionMode::ReadOnly);
    assert!(transaction.is_ok());
    let transaction = transaction.unwrap();

    let nodelist = transaction.store("domnodes");
    assert!(nodelist.is_ok());
    let nodelist = nodelist.unwrap();

    nodelist.count(key_range).await
}
// add context
async fn clear_node_records(rexie: &Rexie) -> Result<()> {
    let transaction = rexie.transaction(&["domnodes"], TransactionMode::ReadWrite);
    assert!(transaction.is_ok());
    let transaction = transaction.unwrap();

    let node_records = transaction.store("domnodes");
    assert!(node_records.is_ok());
    let node_records = node_records.unwrap();

    node_records.clear().await
}

#[component]
async fn MyDomNodes<G: Html>(cx: Scope<'_>) -> View<G> {
    let idb = dom_db().await;
    //let iidb = Rc::new(idb);
    clear_node_records(&idb).await.unwrap();

    insert_node(&idb, "h3", "This Text", 0).await.unwrap();
    insert_node(&idb, "h1", "Another Text Text", 1)
        .await
        .unwrap();

    // let btn_insert_node = insert_node(&mut idb, "h3", "This Text", 4).await.unwrap();

    // let idb = Rc::new(idb);
    // let idb = idb.clone();
    let idb = create_ref(cx, idb);
    let mut btn_click = move |_| {
        spawn_local_scoped(cx, async move {
            // let _ = btn_insert_node;
            // let idb = &idb;
            insert_node(&idb, "h3", "This Text", 4).await.unwrap();
        })
    };

    let count: u32 = count_node_records(&idb, None).await.unwrap();

    let node_list = fetch_json_nodes(&idb, None).await.unwrap();

    let nlist = create_signal(cx, node_list);
    let last_inserted_node = create_signal(
        cx,
        insert_node(&idb, "h3", "This Text", 0)
            .await
            .unwrap_or_default(),
    );

    view! { cx,
        ul {
            Keyed {
                iterable: nlist,
                view: |cx, x| view! { cx,
                    // li { (x.tag) }

                     (View::new_node(
                         {
                             let el = G::element_from_tag(&x.tag);
                             el.append_child(&G::text_node(&x.text));
                             el
                             }))
                }

                ,
                key: |x| x.id,
            }

        }
            button(on:click=btn_click ){"insert"}
    }
}

#[component]
fn App<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        div {
            "Component demo"

            // NodeAdd()

                MyDomNodes()

        }
    }
}

fn main() {
    sycamore::render(|cx| {
        view! { cx, App() }
    });
}
