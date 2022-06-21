use rexie::*;
use serde::Deserialize;
use serde::Serialize;
// use std::rc::Rc;
// use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::{use_transition, Suspense};
use uuid::Uuid;
use wasm_bindgen::prelude::*;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonFrontDom {
    pub uuid: Uuid,
    pub tag: String,
    pub text: String,
    pub order: u16,
}

// -
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonNode {
    pub d_uuid: Uuid,
    pub d_tag: String,
    pub d_text: String,
    pub order: u16,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonDom<'a> {
    pub uuid: Uuid,
    pub tag: &'a str,
    pub text: &'a str,
    pub order: u16,
}

//-

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomRecord {
    pub id: u32,
    pub uuid: Uuid,
    pub tag: String,
    pub text: String,
    pub order: u16,
}

#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub contents: RcSignal<Vec<RcSignal<DomRecord>>>,
}

//-
impl AppState {
    fn add_node(&self, tag: String, text: String) {
        self.contents.modify().push(create_rc_signal(DomRecord {
            id: todo!(),
            uuid: Uuid::new_v4(),
            tag,
            text,
            order: todo!(),
        }))
    }

    // fn remove_todo(&self, id: Uuid) {
    //     self.todos.modify().retain(|todo| todo.get().id != id);
    // }

    // fn clear_completed(&self) {
    //     self.todos.modify().retain(|todo| !todo.get().completed);
    // }
}

pub type DomNodes = Vec<JsonFrontDom>;

/// Creates a database
async fn dom_db() -> Rexie {
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

async fn node_query(
    rexie: &Rexie,
    direction: Option<Direction>,
) -> Result<Vec<RcSignal<DomRecord>>> {
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

    let node_records: Vec<RcSignal<DomRecord>> = node_records
        .into_iter()
        // .map(|node_record| serde_wasm_bindgen::from_value(node_record).unwrap())
        .map(|node_record| create_rc_signal(serde_wasm_bindgen::from_value(node_record).unwrap()))
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
async fn App<G: Html>(cx: Scope<'_>) -> View<G> {
    // Initialize application state

    let idb = dom_db().await;
    clear_node_records(&idb).await.unwrap();
    insert_node(&idb, "h3", "This Text", 0).await.unwrap();
    insert_node(&idb, "h1", "Another Text Text", 1)
        .await
        .unwrap();

    let node_list = node_query(&idb, None).await.unwrap();

    let contents = create_rc_signal(node_list);

    let app_state = AppState { contents };

    provide_context(cx, app_state);

    create_effect(cx, move || {
        let app_state = use_context::<AppState>(cx);
        for content in app_state.contents.get().iter() {
            content.track();
            //log::debug!("Content -> {:#?}", content.get());
        }
    });

    view! { cx,
        div(class="todomvc-wrapper") {
            // section(class="todoapp") {
            //     Header {}
            //     List {}
            //     Footer {}
            // }
            p(){"Welcome"}
            Copyright {}
            TextNodes{}
        }
    }
}

#[component]
fn TextNodes<G: Html>(cx: Scope<'_>) -> View<G> {
    let app_state = use_context::<AppState>(cx);

    let node_vect = create_memo(cx, || {
        app_state
            .contents
            .get()
            .iter()
            .map(|content| content.get())
            .collect::<Vec<_>>()
    });

    view! { cx,
                            p(){(format!("----"))}

            ul {
                Keyed {
                    iterable: node_vect,
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

    //

        }
}

#[component]
fn EditableDiv<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
          p {
              "Editable Div"
          }

              div (class="content-area") {
      div (class="visuell-view", contenteditable=true) {"eeeeee"}
    }

      }
}
// updateyyy

#[component]
pub fn Copyright<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        footer(class="info") {
            p { "Double click to edit a todo" }
            p {
                "Created by "
                a(href="https://github.com/lukechu10", target="_blank") { "lukechu10" }
            }
            p {
                "Part of "
                a(href="http://todomvc.com") { "TodoMVC" }
            }
        }
    }
}

// #[component]
// fn App<G: Html>(cx: Scope) -> View<G> {
//     view! { cx,
//         div {
//             "Component demo"

//             // NodeAdd()

//                 MyDomNodes()

//         }
//     }
// }

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();
    sycamore::render(|cx| {
        view! { cx, App() }
    });
}
