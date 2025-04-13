pub fn file_url(session: &str, device: &str) -> String {
    format!("./files/{}_{}.hdf5", device, session)
}