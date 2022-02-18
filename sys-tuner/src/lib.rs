use log::*;

pub const renec_sys_tuner_PATH: &str = "/tmp/renec-sys-tuner";

#[cfg(unix)]
pub fn request_realtime_poh() {
    info!("Sending tuning request");
    let status = unix_socket::UnixStream::connect(renec_sys_tuner_PATH);
    match status {
        Ok(_) => info!("Successfully sent tuning request"),
        Err(err) => warn!(
            "Failed to send tuning request, is `renec-sys-tuner` running? {:?}",
            err
        ),
    }
}

#[cfg(not(unix))]
pub fn request_realtime_poh() {
    info!("Tuning request ignored on this platform");
}
