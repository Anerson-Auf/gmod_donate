mod tcp;

use tcp::TcpClient;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::Mutex;

use gmod::lua::State;
use gmod_tcp_shared::types::Message;
use std::ffi::CString;

#[macro_use] extern crate gmod;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static MESSAGE_QUEUE: OnceLock<Mutex<Vec<Message>>> = OnceLock::new();
static CLIENT: OnceLock<Arc<TcpClient>> = OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new().expect("Failed to create tokio runtime")
    })
}

fn get_message_queue() -> &'static Mutex<Vec<Message>> {
    MESSAGE_QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

fn get_client() -> &'static Arc<TcpClient> {
    CLIENT.get().expect("Client not initialized")
}

fn push_messages_to_lua(lua: State, messages: &[Message]) {
    unsafe {
        lua.new_table();
        for (i, message) in messages.iter().enumerate() {
            lua.push_integer((i + 1) as isize);
            lua.new_table();
            
            lua.push_string("id");
            lua.push_integer(message.id as isize);
            lua.set_table(-3);
            
            lua.push_string("client_uuid");
            lua.push_string(&message.client_uuid);
            lua.set_table(-3);
            
            lua.push_string("message_type");
            lua.push_string(&message.message_type);
            lua.set_table(-3);
            
            lua.push_string("status");
            lua.push_string(&message.status);
            lua.set_table(-3);
            
            lua.push_string("created_at");
            lua.push_string(&message.created_at.to_rfc3339());
            lua.set_table(-3);
            
            if let Some(delivered_at) = message.delivered_at {
                lua.push_string("delivered_at");
                lua.push_string(&delivered_at.to_rfc3339());
                lua.set_table(-3);
            }
            
            lua.push_string("message_data");
            let data_str = serde_json::to_string(&message.message_data).unwrap_or_else(|_| "{}".to_string());
            lua.push_string(&data_str);
            lua.set_table(-3);
            
            lua.set_table(-3);
        }
    }
}

unsafe extern "C-unwind" fn get_messages(lua: State) -> i32 {
    let queue = get_message_queue();
    let mut messages = queue.lock().unwrap();

    if messages.is_empty() {
        unsafe {
            lua.push_nil();
        }
        return 1;
    }
    
    push_messages_to_lua(lua, &messages);
    
    messages.clear();
    1
}

unsafe extern "C-unwind" fn poll_now(lua: State) -> i32 {
    let rt = get_runtime();
    let client = Arc::clone(get_client());
    let queue = get_message_queue();
    
    rt.spawn(async move {
        match client.find_messages().await {
            Ok(messages) => {
                if !messages.is_empty() {
                    let mut queue_guard = queue.lock().unwrap();
                    queue_guard.extend(messages);
                    println!("PollNow: Added {} message(s) to queue", queue_guard.len());
                } else {
                    println!("PollNow: No new messages");
                }
            }
            Err(e) => {
                eprintln!("PollNow: Failed to poll messages: {}", e);
            }
        }
    });
    
    unsafe {
        lua.push_boolean(true);
    }
    1
}

#[gmod13_open]
fn gmod13_open(state: State) -> i32 {
    println!("Module start loading!");

    unsafe {
        state.new_table();
        state.set_global(CString::new("GModTCPMessages").unwrap().as_ptr());
        state.push_function(get_messages);
        state.set_global(CString::new("GModTCPGetMessages").unwrap().as_ptr());
        state.push_function(poll_now);
        state.set_global(CString::new("GModTCPPollNow").unwrap().as_ptr());
    }
    
    let rt = get_runtime();
    
    let client = match rt.block_on(async {
        TcpClient::new().await
    }) {
        Ok(client) => Arc::new(client),
        Err(e) => {
            eprintln!("Failed to create TCP client: {}", e);
            return 1;
        }
    };
    
    println!("Client UUID: {}", client.client_uuid);
    println!("Registering client on server");
    
    if let Err(e) = rt.block_on(client.register()) {
        eprintln!("Failed to register client: {}", e);
        return 1;
    }
    
    println!("Client registered successfully");
    println!("Starting polling loop (every 10 minutes)");
    
    CLIENT.set(Arc::clone(&client)).expect("Failed to set client");
    
    let message_queue = get_message_queue();
    if let Err(e) = rt.block_on(client.listen(message_queue)) {
        eprintln!("Failed to start listening: {}", e);
        return 1;
    }
    
    println!("Module loaded successfully!");
    0
}

#[gmod13_close]
fn gmod13_close(_state: State) -> i32 {
    0
}