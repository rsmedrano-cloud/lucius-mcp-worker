use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::time::{self, Duration};

// --- Structs and Enums ---
#[derive(Serialize, Deserialize, Debug)]
struct Task {
    id: String,
    target_host: String,
    task_type: TaskType,
    details: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
enum TaskType {
    DOCKER,
    SHELL,
}

// --- Logging ---
fn log(msg: &str) {
    println!("{}", msg);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("mcp-worker.log")
    {
        writeln!(file, "{}", msg).ok();
        file.flush().ok();
    }
}

// --- Main Application Logic ---
#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    log("--- MCP-WORKER START ---");

    let redis_host = env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let redis_url = format!("redis://{}/", redis_host);

    let client = match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(e) => {
            log(&format!("FATAL: Redis client creation failed: {}", e));
            return;
        }
    };

    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            log(&format!("FATAL: Failed to get multiplexed Redis connection: {}", e));
            return;
        }
    };

    log("Successfully connected to Redis. Entering command listener loop...");
    command_listener(&mut conn).await;
}

async fn command_listener(conn: &mut redis::aio::MultiplexedConnection) {
    let queue_key = "mcp::tasks::all";
    log(&format!("Listening for commands on '{}'", queue_key));

    loop {
        // 1. Safe Pop from the queue
        let pop_result: redis::RedisResult<Option<String>> = conn.lpop(queue_key, None).await;

        match pop_result {
            Ok(Some(json_str)) => {
                log(&format!(">>> RECEIVED: {}", json_str));

                // 2. Safe Parse the JSON into a Task
                match serde_json::from_str::<Task>(&json_str) {
                    Ok(task) => {
                        log(&format!("Processing Task ID: {}", task.id));
                        
                        // 3. Execute the task based on its type
                        let task_result = execute_task(&task).await;
                        
                        // 4. Write the result back to Redis
                        let res_key = format!("mcp::result::{}", task.id);
                        let res_val = match task_result {
                            Ok(output) => format!("SUCCESS: {}", output),
                            Err(e) => format!("ERROR: {}", e),
                        };

                        let _: redis::RedisResult<()> = conn.set_ex(&res_key, res_val, 3600).await;
                        log(&format!("Result for task {} written to Redis.", task.id));
                    }
                    Err(e) => {
                        log(&format!("[ERROR] JSON Parse Error: {}", e));
                    }
                }
            }
            Ok(None) => {
                // This is the normal case when the queue is empty.
                // Sleep for a short duration to prevent busy-looping.
                time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                log(&format!("[ERROR] Redis Error in Loop: {:?}", e));
                // If a Redis error occurs, wait a bit before retrying.
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn execute_task(task: &Task) -> Result<String, String> {
    log(&format!("Executing task type: {:?}", task.task_type));
    match task.task_type {
        TaskType::SHELL => {
            // Mock execution for now
            log("TaskType was SHELL. (Not implemented, mock success)");
            Ok("Shell command executed successfully.".to_string())
        }
        TaskType::DOCKER => {
            // Mock execution for now
            log("TaskType was DOCKER. (Not implemented, mock success)");
            Ok("Docker command executed successfully.".to_string())
        }
    }
}