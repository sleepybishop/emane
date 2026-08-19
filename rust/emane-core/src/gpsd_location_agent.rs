use std::os::raw::{c_char, c_void, c_int, c_double};
use std::ffi::{CString, CStr};
use std::ptr;
use std::fs;
use libc::{open, grantpt, unlockpt, posix_openpt, ptsname_r, O_RDWR, O_NOCTTY, read, write, time, gmtime_r, time_t, tm, unlink};

pub struct GpsdLocationAgent {
    nem_id: u16,
    master_pty: i32,
    pseudo_terminal_file: String,
    have_initial_position: bool,
    have_initial_velocity: bool,
    lat_degrees: f64,
    lon_degrees: f64,
    alt_meters: f64,
    azm_degrees: f64,
    mag_mps: f64,
}

impl GpsdLocationAgent {
    pub fn new(nem_id: u16) -> Self {
        Self {
            nem_id,
            master_pty: -1,
            pseudo_terminal_file: String::new(),
            have_initial_position: false,
            have_initial_velocity: false,
            lat_degrees: 0.0,
            lon_degrees: 0.0,
            alt_meters: 0.0,
            azm_degrees: 0.0,
            mag_mps: 0.0,
        }
    }

    pub fn start(&mut self, pseudo_terminal_file: &str) {
        self.pseudo_terminal_file = pseudo_terminal_file.to_string();
        
        unsafe {
            self.master_pty = posix_openpt(O_RDWR | O_NOCTTY);
            if self.master_pty < 0 {
                panic!("posix_openpt failed");
            }

            if grantpt(self.master_pty) < 0 {
                panic!("grantpt failed");
            }

            if unlockpt(self.master_pty) < 0 {
                panic!("unlockpt failed");
            }

            let mut pts_name = [0i8; 1024];
            if ptsname_r(self.master_pty, pts_name.as_mut_ptr(), 1024) != 0 {
                panic!("ptsname_r failed");
            }
            
            let pts_str = CStr::from_ptr(pts_name.as_ptr()).to_string_lossy().into_owned();
            
            // Create symlink
            let _ = std::fs::remove_file(&self.pseudo_terminal_file);
            if std::os::unix::fs::symlink(&pts_str, &self.pseudo_terminal_file).is_err() {
                panic!("symlink failed");
            }
        }
    }

    pub fn stop(&mut self) {
        if self.master_pty >= 0 {
            unsafe { libc::close(self.master_pty); }
            self.master_pty = -1;
        }
        let _ = std::fs::remove_file(&self.pseudo_terminal_file);
    }
    
    pub fn do_checksum_nmea(buf: &mut String) {
        let mut chksum: u8 = 0;
        let bytes = buf.as_bytes();
        
        if bytes.len() > 1 && bytes[0] == b'$' {
            for i in 1..bytes.len() {
                chksum ^= bytes[i];
            }
        }
        
        buf.push_str(&format!("*{:02X}\r\n", chksum));
    }
    
    pub fn write_pty(&self, buf: &str) {
        if self.master_pty >= 0 {
            unsafe {
                write(self.master_pty, buf.as_ptr() as *const c_void, buf.len());
            }
        }
    }

    pub fn send_spoofed_nmea(&self, mut lat: f64, mut lon: f64, alt: f64) {
        let mut t: time_t = 0;
        let mut tmval: tm = unsafe { std::mem::zeroed() };
        unsafe {
            time(&mut t);
            gmtime_r(&t, &mut tmval);
        }

        let num_gsv_strings = 2;
        let num_sv_per_string = 4;
        let elv = [41, 9, 70, 35, 10, 53, 2, 48];
        let azm = [104, 84, 30, 185, 297, 311, 29, 64];
        let snr = [41, 51, 39, 25, 25, 21, 29, 32];

        let lat_hemisphere = if lat > 0.0 { 'N' } else { 'S' };
        let lon_hemisphere = if lon > 0.0 { 'E' } else { 'W' };

        if lat < 0.0 { lat = -lat; }
        if lon < 0.0 { lon = -lon; }

        let lat_deg = lat as i32;
        let lon_deg = lon as i32;

        let mut lat_min = (lat - lat_deg as f64) * 60.0;
        let mut lon_min = (lon - lon_deg as f64) * 60.0;

        let lat_min_int = lat_min as i32;
        let lon_min_int = lon_min as i32;

        lat_min -= lat_min_int as f64;
        lon_min -= lon_min_int as f64;

        lat_min *= 10000.0;
        lon_min *= 10000.0;

        let mut buf = format!(
            "$GPGGA,{:02}{:02}{:02},{:02}{:02}.{:04},{},{:03}{:02}.{:04},{},2,{:02},1.1,{:.1},M,-34.0,M,,",
            tmval.tm_hour, tmval.tm_min, tmval.tm_sec,
            lat_deg, lat_min_int, lat_min as i32, lat_hemisphere,
            lon_deg, lon_min_int, lon_min as i32, lon_hemisphere,
            num_gsv_strings * num_sv_per_string, alt
        );
        Self::do_checksum_nmea(&mut buf);
        self.write_pty(&buf);

        let mut buf = format!(
            "$GPRMC,{:02}{:02}{:02},A,{:02}{:02}.{:04},{},{:03}{:02}.{:04},{},000.0,000.0,{:02}{:02}{:02},000.0,{}",
            tmval.tm_hour, tmval.tm_min, tmval.tm_sec,
            lat_deg, lat_min_int, lat_min as i32, lat_hemisphere,
            lon_deg, lon_min_int, lon_min as i32, lon_hemisphere,
            tmval.tm_mday, tmval.tm_mon + 1, (tmval.tm_year + 1900) % 100,
            lon_hemisphere
        );
        Self::do_checksum_nmea(&mut buf);
        self.write_pty(&buf);

        let mut buf = format!("$GPGSA,A,3,01,02,03,04,05,06,07,08,,,,,1.8,1.1,1.3");
        Self::do_checksum_nmea(&mut buf);
        self.write_pty(&buf);

        for i in 0..num_gsv_strings {
            let a = num_sv_per_string * i;
            let b = a + 1;
            let c = a + 2;
            let d = a + 3;

            let mut buf = format!(
                "$GPGSV,{},{},{:02},{:02},{:02},{:03},{:02},{:02},{:02},{:03},{:02},{:02},{:02},{:03},{:02},{:02},{:02},{:03},{:02}",
                num_gsv_strings, i + 1, num_gsv_strings * num_sv_per_string,
                a + 1, elv[a], azm[a], snr[a],
                b + 2, elv[b], azm[b], snr[b],
                c + 3, elv[c], azm[c], snr[c],
                d + 4, elv[d], azm[d], snr[d]
            );
            Self::do_checksum_nmea(&mut buf);
            self.write_pty(&buf);
        }
    }

    pub fn send_spoofed_gpvtg(&self, azm: f64, mag: f64) {
        let mag_knots = mag * 1.94384;
        let mag_kph = (mag * 3600.0) / 1000.0;

        let mut buf = format!(
            "$GPVTG,{:.1},T,{:.1},M,{:.1},N,{:.1},K",
            azm, azm, mag_knots, mag_kph
        );
        Self::do_checksum_nmea(&mut buf);
        self.write_pty(&buf);
    }
}

// C FFI Interface
#[no_mangle]
pub extern "C" fn emane_rs_gpsd_agent_new(nem_id: u16) -> *mut GpsdLocationAgent {
    Box::into_raw(Box::new(GpsdLocationAgent::new(nem_id)))
}

#[no_mangle]
pub extern "C" fn emane_rs_gpsd_agent_free(ptr: *mut GpsdLocationAgent) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_gpsd_agent_start(ptr: *mut GpsdLocationAgent, pty_file: *const c_char) {
    let agent = unsafe { &mut *ptr };
    let file = unsafe { CStr::from_ptr(pty_file) }.to_string_lossy().into_owned();
    agent.start(&file);
}

#[no_mangle]
pub extern "C" fn emane_rs_gpsd_agent_stop(ptr: *mut GpsdLocationAgent) {
    let agent = unsafe { &mut *ptr };
    agent.stop();
}

#[no_mangle]
pub extern "C" fn emane_rs_gpsd_agent_update_location(
    ptr: *mut GpsdLocationAgent,
    lat: c_double,
    lon: c_double,
    alt: c_double,
    has_velocity: bool,
    azm: c_double,
    mag: c_double,
) {
    let agent = unsafe { &mut *ptr };
    agent.lat_degrees = lat;
    agent.lon_degrees = lon;
    agent.alt_meters = alt;
    agent.have_initial_position = true;

    if has_velocity {
        agent.have_initial_velocity = true;
        agent.azm_degrees = azm;
        agent.mag_mps = mag;
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_gpsd_agent_process_timed_event(ptr: *mut GpsdLocationAgent) {
    let agent = unsafe { &mut *ptr };
    if agent.have_initial_position {
        agent.send_spoofed_nmea(agent.lat_degrees, agent.lon_degrees, agent.alt_meters);
        if agent.have_initial_velocity {
            agent.send_spoofed_gpvtg(agent.azm_degrees, agent.mag_mps);
        }
    }
}
