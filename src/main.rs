use nanoserde::{DeJson, SerJson};
use std::collections::HashSet;
use std::env;
use std::fs;
use chrono::{Local, Duration};

const SENT_APPOINTMENTS_FILE: &str = "sent_appointments.json";
const DEFAULT_DAYS_AHEAD: i32 = 14;
const GRAPHQL_QUERY: &str = r#"query GetAvailableAppointmentSlots($storeNumbers: [String!]!, $slotsQuery: AvailableSlotsQueryInput!, $lineOfBusiness: LineOfBusiness!) {
  storeAppointmentSlots(
    storeNumbers: $storeNumbers
    lineOfBusiness: $lineOfBusiness
  ) {
    availableSlots(query: $slotsQuery) {
      date
      count
      appointmentSlots {
        id
        clinicId
        slotType
        startTime
        endTime
        __typename
      }
      __typename
    }
    __typename
  }
}"#;

#[derive(DeJson, SerJson, Clone, Debug)]
struct AppointmentSlot {
    id: String,
    #[nserde(rename = "clinicId")]
    clinic_id: Option<String>,
    #[nserde(rename = "slotType")]
    slot_type: String,
    #[nserde(rename = "startTime")]
    start_time: String,
    #[nserde(rename = "endTime")]
    end_time: String,
}

#[derive(DeJson, Debug)]
struct AvailableSlots {
    date: String,
    count: i32,
    #[nserde(rename = "appointmentSlots")]
    appointment_slots: Vec<AppointmentSlot>,
}

#[derive(DeJson, Debug)]
struct StoreAppointmentSlots {
    #[nserde(rename = "availableSlots")]
    available_slots: Vec<AvailableSlots>,
}

#[derive(DeJson, Debug)]
struct GraphQLData {
    #[nserde(rename = "storeAppointmentSlots")]
    store_appointment_slots: Vec<StoreAppointmentSlots>,
}

#[derive(DeJson, Debug)]
struct GraphQLResponse {
    data: GraphQLData,
}

fn build_graphql_request(start_date: &str, end_date: &str, store_number: &str, days_ahead: i32) -> String {
    format!(
        r#"{{"operationName":"GetAvailableAppointmentSlots","variables":{{"lineOfBusiness":"OPTICAL","slotsQuery":{{"maxEndDate":"{}","maxNumberOfDays":{},"slotType":"ADULT_EYE_TEST","startDate":"{}"}},"storeNumbers":["{}"]}},"query":"{}"}}"#,
        end_date,
        days_ahead,
        start_date,
        store_number,
        GRAPHQL_QUERY.replace('\n', "\\n").replace('"', "\\\"")
    )
}

#[derive(SerJson)]
struct DiscordMessage<'a> {
    content: &'a str,
}

#[derive(DeJson, SerJson)]
struct SentAppointments {
    ids: Vec<String>,
}

fn load_sent_appointments() -> HashSet<String> {
    match fs::read_to_string(SENT_APPOINTMENTS_FILE) {
        Ok(content) => {
            if let Ok(sent) = SentAppointments::deserialize_json(&content) {
                sent.ids.into_iter().collect()
            } else {
                HashSet::new()
            }
        }
        Err(_) => HashSet::new(),
    }
}

fn save_sent_appointments(ids: &HashSet<String>) -> Result<(), String> {
    let sent = SentAppointments {
        ids: ids.iter().cloned().collect(),
    };
    let json = sent.serialize_json();
    fs::write(SENT_APPOINTMENTS_FILE, json).map_err(|e| e.to_string())
}

fn get_date_range(days_ahead: i32) -> (String, String) {
    let today = Local::now().date_naive();
    let end_date = today + Duration::days(days_ahead as i64);

    let start_date = today.format("%Y-%m-%d").to_string();
    let end_date = end_date.format("%Y-%m-%d").to_string();

    (start_date, end_date)
}

fn fetch_appointments(graphql_url: &str, start_date: &str, end_date: &str, store_number: &str, days_ahead: i32) -> Result<Vec<AppointmentSlot>, String> {
    let body = build_graphql_request(start_date, end_date, store_number, days_ahead);

    let response = minreq::post(graphql_url)
        .with_header("Content-Type", "application/json")
        .with_header("x-specsavers-application-id", "nuxt-find-and-book")
        .with_header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .with_header("Accept", "application/json, text/plain, */*")
        .with_header("Accept-Language", "en-GB,en;q=0.9")
        .with_header("Origin", "https://www.specsavers.ie")
        .with_header("Referer", "https://www.specsavers.ie/")
        .with_body(body)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let response_text = response.as_str().map_err(|e| format!("Failed to read response: {}", e))?;

    let graphql_response = GraphQLResponse::deserialize_json(response_text)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let mut all_slots = Vec::new();
    for store_slots in graphql_response.data.store_appointment_slots {
        for available in store_slots.available_slots {
            all_slots.extend(available.appointment_slots);
        }
    }

    Ok(all_slots)
}

fn send_to_discord(webhook_url: &str, appointments: &[AppointmentSlot]) -> Result<(), String> {
    let mut message = String::from("**New SpecSavers Appointments Available!**\n\n");

    for appt in appointments {
        message.push_str(&format!(
            "📅 **{}** - {} to {}\n",
            appt.start_time, appt.start_time, appt.end_time
        ));
    }

    let discord_msg = DiscordMessage {
        content: &message,
    };

    let body = discord_msg.serialize_json();

    minreq::post(webhook_url)
        .with_header("Content-Type", "application/json")
        .with_body(body)
        .send()
        .map_err(|e| format!("Failed to send Discord message: {}", e))?;

    Ok(())
}

fn main() {
    let webhook_url = match env::var("DISCORD_WEBHOOK_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("Error: DISCORD_WEBHOOK_URL environment variable not set");
            std::process::exit(1);
        }
    };

    let graphql_url = env::var("GRAPHQL_URL")
        .unwrap_or_else(|_| "https://www.specsavers.ie/graphql".to_string());

    let store_number = env::var("STORE_NUMBER").unwrap_or_else(|_| "284".to_string());

    let days_ahead = env::var("DAYS_AHEAD")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(DEFAULT_DAYS_AHEAD);

    println!("Checking SpecSavers appointments...");

    let (start_date, end_date) = get_date_range(days_ahead);
    println!("Checking from {} to {}", start_date, end_date);

    let appointments = match fetch_appointments(&graphql_url, &start_date, &end_date, &store_number, days_ahead) {
        Ok(appts) => appts,
        Err(e) => {
            eprintln!("Error fetching appointments: {}", e);
            std::process::exit(1);
        }
    };

    println!("Found {} total appointment slots", appointments.len());

    if appointments.is_empty() {
        println!("No appointments available.");
        return;
    }

    let mut sent_ids = load_sent_appointments();
    let new_appointments: Vec<AppointmentSlot> = appointments
        .into_iter()
        .filter(|appt| !sent_ids.contains(&appt.id))
        .collect();

    println!("Found {} new appointments", new_appointments.len());

    if new_appointments.is_empty() {
        println!("No new appointments to report.");
        return;
    }

    println!("Sending {} new appointments to Discord...", new_appointments.len());

    if let Err(e) = send_to_discord(&webhook_url, &new_appointments) {
        eprintln!("Error sending to Discord: {}", e);
        std::process::exit(1);
    }

    for appt in &new_appointments {
        sent_ids.insert(appt.id.clone());
    }

    if let Err(e) = save_sent_appointments(&sent_ids) {
        eprintln!("Error saving sent appointments: {}", e);
        std::process::exit(1);
    }

    println!("Successfully sent {} new appointments!", new_appointments.len());
}
