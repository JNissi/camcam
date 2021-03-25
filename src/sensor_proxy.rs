use zbus::dbus_proxy;

#[dbus_proxy(
    interface = "net.hadess.SensorProxy",
    default_service = "net.hadess.SensorProxy",
    default_path = "/net/hadess/SensorProxy")]
trait SensorProxy {
    /// ClaimAccelerometer method
    fn claim_accelerometer(&self) -> zbus::Result<()>;

    /// ClaimLight method
    fn claim_light(&self) -> zbus::Result<()>;

    /// ClaimProximity method
    fn claim_proximity(&self) -> zbus::Result<()>;

    /// ReleaseAccelerometer method
    fn release_accelerometer(&self) -> zbus::Result<()>;

    /// ReleaseLight method
    fn release_light(&self) -> zbus::Result<()>;

    /// ReleaseProximity method
    fn release_proximity(&self) -> zbus::Result<()>;

    /// AccelerometerOrientation property
    #[dbus_proxy(property)]
    fn accelerometer_orientation(&self) -> zbus::Result<String>;

    /// HasAccelerometer property
    #[dbus_proxy(property)]
    fn has_accelerometer(&self) -> zbus::Result<bool>;

    /// HasAmbientLight property
    #[dbus_proxy(property)]
    fn has_ambient_light(&self) -> zbus::Result<bool>;

    /// HasProximity property
    #[dbus_proxy(property)]
    fn has_proximity(&self) -> zbus::Result<bool>;

    /// LightLevel property
    #[dbus_proxy(property)]
    fn light_level(&self) -> zbus::Result<f64>;

    /// LightLevelUnit property
    #[dbus_proxy(property)]
    fn light_level_unit(&self) -> zbus::Result<String>;

    /// ProximityNear property
    #[dbus_proxy(property)]
    fn proximity_near(&self) -> zbus::Result<bool>;
}

