#[derive(Debug, dust_dds::topic_definition::type_support::DdsType)]
pub struct CabinLeftWindowStatus {
    pub command: i32,
    pub status: i32,
    pub progress: i32,
    pub uistatus: i32,
}

#[derive(Debug, dust_dds::topic_definition::type_support::DdsType)]
pub struct CabinRightWindowStatus {
    pub command: i32,
    pub status: i32,
    pub progress: i32,
    pub uistatus: i32,
}

impl crate::Piccoloable for CabinLeftWindowStatus {
    fn to_piccolo_dds_data(&self) -> crate::DdsData {
        let lwindow = match self.uistatus {
            // uistatus: 0 = IS_ACTING, 1 = CHARGING_PORT_IS_CLOSED, 2 = CHARGING_PORT_IS_OPENED
            0 => "is_acting",
            1 => "close",
            2 => "open",
            _ => "unknown",
        };

        crate::DdsData {
            name: String::from("CabinLeftWindowStatus"),
            value: lwindow.to_string(),
        }
    }
    fn topic_name() -> String {
        String::from("CabinLeftWindowStatus")
    }
    fn type_name() -> String {
        String::from("CabinLeftWindowStatus")
    }
}

impl crate::Piccoloable for CabinRightWindowStatus {
    fn to_piccolo_dds_data(&self) -> crate::DdsData {
        let rwindow = match self.uistatus {
            // uistatus: 0 = IS_ACTING, 1 = CHARGING_PORT_IS_CLOSED, 2 = CHARGING_PORT_IS_OPENED
            0 => "is_acting",
            1 => "close",
            2 => "open",
            _ => "unknown",
        };

        crate::DdsData {
            name: String::from("CabinRightWindowStatus"),
            value: rwindow.to_string(),
        }
    }
    fn topic_name() -> String {
        String::from("CabinRightWindowStatus")
    }
    fn type_name() -> String {
        String::from("CabinRightWindowStatus")
    }
}
