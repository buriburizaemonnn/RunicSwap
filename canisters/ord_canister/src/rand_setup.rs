fn alway_fail(_b: &mut [u8]) -> Result<(), getrandom::Error> {
  Err(getrandom::Error::UNSUPPORTED)
}

getrandom::register_custom_getrandom!(alway_fail);
