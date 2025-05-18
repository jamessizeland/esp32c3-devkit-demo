use bt_hci::controller::ExternalController;
use esp_wifi::ble::controller::BleConnector;
pub use gatt::GattServer;
use log::info;
// use log::info;
use static_cell::StaticCell;
use trouble_host::prelude::*;

mod gatt;

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att

/// Max L2CAP MTU size.
const L2CAP_MTU: usize = 256;

const SLOTS: usize = 20;
pub type BleController = ExternalController<BleConnector<'static>, SLOTS>;

pub type BleResources = HostResources<CONNECTIONS_MAX, L2CAP_CHANNELS_MAX, L2CAP_MTU>;

#[embassy_executor::task]
async fn ble_task(mut runner: Runner<'static, BleController>) {
    runner.run().await.expect("Error in BLE task");
}

impl<'d> GattServer<'d> {
    /// Build the stack for the GATT server and start background tasks required.
    pub fn start(
        name: &'d str,
        spawner: embassy_executor::Spawner,
        controller: BleController,
    ) -> (&'static Self, Peripheral<'d, BleController>) {
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
                    appearance: &appearance::human_interface_device::GAMEPAD,
                }))
                .expect("Error creating Gatt Server"),
            )
        };
        info!("Starting Gatt Server");
        spawner.must_spawn(ble_task(host.runner));
        (server, host.peripheral)
    }

    /// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
    pub async fn advertise<'a, C: Controller>(
        name: &'a str,
        peripheral: &mut Peripheral<'a, C>,
    ) -> Result<Connection<'a>, BleHostError<C::Error>> {
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
        let conn = advertiser.accept().await?;
        info!("[adv] connection established");
        Ok(conn)
    }
}
