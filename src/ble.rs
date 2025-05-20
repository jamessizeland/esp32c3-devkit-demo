use esp_wifi::ble::controller::BleConnector;
pub use gatt::GattServer;
use log::{info, warn};
use static_cell::StaticCell;
use trouble_host::prelude::*;

mod gatt;
mod notify;

/// Maximum number of connections
const CONN_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att

/// Max L2CAP MTU size.
const L2CAP_MTU: usize = 256;

const SLOTS: usize = 20;

pub type BleController = bt_hci::controller::ExternalController<BleConnector<'static>, SLOTS>;

type BleResources = HostResources<CONN_MAX, L2CAP_CHANNELS_MAX, L2CAP_MTU>;

/// Helper type that combines the long lived server struct with the more ephemeral current connection.
pub type BleConnection<'values, 'server> = (
    &'server GattServer<'values>,
    &'server GattConnection<'values, 'server>,
);

#[embassy_executor::task]
async fn ble_task(mut runner: Runner<'static, BleController>) {
    runner.run().await.expect("Error in BLE task");
}

/// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
pub async fn advertise<'server, 'values, C: Controller>(
    name: &'values str,
    peripheral: &mut Peripheral<'values, C>,
    server: &'server GattServer<'values>,
) -> Result<GattConnection<'values, 'server>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x0f, 0x18]]),
            AdStructure::CompleteLocalName(name.as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &advertiser_data[..],
                scan_data: &[],
            },
        )
        .await?;
    info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}

impl<'values> GattServer<'values> {
    /// Build the stack for the GATT server and start background tasks required.
    pub fn start(
        name: &'values str,
        appearance: impl Into<&'static BluetoothUuid16>,
        spawner: embassy_executor::Spawner,
        controller: BleController,
    ) -> (&'static Self, Peripheral<'values, BleController>) {
        let address = Address::random([0x42, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]);
        info!("Our address = {:?}", address);

        let resources = {
            static RESOURCES: StaticCell<BleResources> = StaticCell::new();
            RESOURCES.init(BleResources::new())
        };
        let stack = {
            static STACK: StaticCell<Stack<'_, BleController>> = StaticCell::new();
            STACK.init(trouble_host::new(controller, resources).set_random_address(address))
        };
        let host = stack.build();
        let server = {
            static SERVER: StaticCell<GattServer<'_>> = StaticCell::new();
            SERVER.init(
                GattServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
                    name,
                    appearance: appearance.into(),
                }))
                .expect("Error creating Gatt Server"),
            )
        };
        info!("Starting Gatt Server");
        spawner.must_spawn(ble_task(host.runner));
        (server, host.peripheral)
    }

    /// Background task to process BLE IO events.
    pub async fn start_task<'server>(
        &self,
        conn: &GattConnection<'values, 'server>,
    ) -> Result<(), trouble_host::Error> {
        let reason = loop {
            match conn.next().await {
                GattConnectionEvent::Disconnected { reason } => break reason,
                GattConnectionEvent::Gatt { event: Err(e) } => {
                    warn!("[gatt] error processing event: {:?}", e)
                }
                GattConnectionEvent::Gatt { event: Ok(event) } => {
                    match &event {
                        GattEvent::Read(_event) => {
                            info!("[gatt] Unhandled Read event occured");
                        }
                        GattEvent::Write(_event) => {
                            info!("[gatt] Unhandled Write event occured");
                        }
                    }
                    match event.accept() {
                        Ok(reply) => reply.send().await,
                        Err(e) => warn!("[gatt] error sending response: {:?}", e),
                    }
                }
                _ => {} // ignore other events
            }
        };
        info!("[gatt] disconnected: {:?}", reason);
        Ok(())
    }
}
