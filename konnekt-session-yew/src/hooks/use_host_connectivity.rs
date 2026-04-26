use chrono::Utc;
use gloo_timers::future::TimeoutFuture;
use yew::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostConnectivityOptions {
    pub enabled: bool,
    pub unreachable_delay_ms: u32,
}

impl Default for HostConnectivityOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            unreachable_delay_ms: 5_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostConnectivityState {
    pub host_unreachable: bool,
    pub last_host_connection_secs: Option<u64>,
}

#[hook]
pub fn use_host_connectivity(
    is_host: bool,
    peer_count: usize,
    options: HostConnectivityOptions,
) -> HostConnectivityState {
    let last_host_connection_secs = use_state(|| None::<u64>);
    let host_unreachable = use_state(|| false);
    let disconnect_epoch = use_state(|| 0u64);

    {
        let last_host_connection_secs = last_host_connection_secs.clone();
        let host_unreachable = host_unreachable.clone();
        let disconnect_epoch = disconnect_epoch.clone();

        use_effect_with(
            (
                is_host,
                peer_count,
                options.enabled,
                options.unreachable_delay_ms,
            ),
            move |(is_host, peer_count, enabled, unreachable_delay_ms)| {
                if !*enabled {
                    host_unreachable.set(false);
                    disconnect_epoch.set(disconnect_epoch.wrapping_add(1));
                } else if !*is_host {
                    if *peer_count > 0 {
                        last_host_connection_secs.set(Some(now_unix_secs()));
                        host_unreachable.set(false);
                        disconnect_epoch.set(disconnect_epoch.wrapping_add(1));
                    } else {
                        let epoch = disconnect_epoch.wrapping_add(1);
                        disconnect_epoch.set(epoch);
                        let disconnect_epoch = disconnect_epoch.clone();
                        let host_unreachable = host_unreachable.clone();
                        let delay_ms = *unreachable_delay_ms;

                        wasm_bindgen_futures::spawn_local(async move {
                            TimeoutFuture::new(delay_ms).await;
                            if *disconnect_epoch == epoch {
                                host_unreachable.set(true);
                            }
                        });
                    }
                } else {
                    host_unreachable.set(false);
                    disconnect_epoch.set(disconnect_epoch.wrapping_add(1));
                }
                || ()
            },
        );
    }

    HostConnectivityState {
        host_unreachable: *host_unreachable,
        last_host_connection_secs: *last_host_connection_secs,
    }
}

fn now_unix_secs() -> u64 {
    Utc::now().timestamp() as u64
}
