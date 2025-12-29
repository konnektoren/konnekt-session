use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::sdk::{
    trace::{self, RandomIdGenerator, Sampler},
    Resource,
};
use opentelemetry::{global, KeyValue};
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub async fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    // Check if telemetry is enabled
    let enable_telemetry = env::var("ENABLE_TELEMETRY")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Create a formatting layer
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_file(true)
        .with_ansi(true)
        .json();

    // Configure EnvFilter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "konnekt_session=debug,tower_http=debug,axum::rejection=trace,axum=trace,warn",
        )
    });

    if enable_telemetry {
        // Set global propagator
        global::set_text_map_propagator(TraceContextPropagator::new());

        let jaeger_endpoint = env::var("JAEGER_ENDPOINT")
            .unwrap_or_else(|_| "http://jaeger:14268/api/traces".to_string());

        // Configure the tracer with isahc client
        let tracer = opentelemetry_jaeger::new_collector_pipeline()
            .with_service_name("konnekt-session")
            .with_endpoint(&jaeger_endpoint)
            .with_isahc()
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::AlwaysOn)
                    .with_id_generator(RandomIdGenerator::default())
                    .with_max_events_per_span(64)
                    .with_max_attributes_per_span(16)
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", "konnekt-session"),
                        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    ])),
            )
            .with_timeout(std::time::Duration::from_secs(2))
            .install_batch(opentelemetry::runtime::Tokio)?;

        // Create a tracing layer with the configured tracer
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

        // Combine all layers including telemetry and install the subscriber
        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .with(telemetry)
            .try_init()?;

        tracing::info!(
            "Telemetry initialized with Jaeger endpoint: {}",
            jaeger_endpoint
        );
    } else {
        // Install subscriber without telemetry
        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .try_init()?;

        tracing::info!("Telemetry disabled");
    }

    Ok(())
}

pub fn shutdown_telemetry() {
    if env::var("ENABLE_TELEMETRY")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false)
    {
        // Ensure all spans are flushed before shutdown
        opentelemetry::global::shutdown_tracer_provider();
    }
}
