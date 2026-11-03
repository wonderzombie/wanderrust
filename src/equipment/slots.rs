macro_rules! slots {
    ( $( $id:ident $label:literal ),+ $(,)?  ) => {

    pub enum SlotType {
        $( $id, )+
    }

$(
    pub struct $id;

    impl EquipmentSlot for $id {
        const TYPE: SlotType = SlotType::$id;

        fn label(&self) -> &'static str {
            $label
        }
    }
)+
    };
}

pub trait EquipmentSlot {
    const TYPE: SlotType;

    fn label(&self) -> &'static str;
}

slots!(
    MainHand "main hand",
    OffHand "off hand",
    Armor "armor",
    Trinket "trinket",
);
