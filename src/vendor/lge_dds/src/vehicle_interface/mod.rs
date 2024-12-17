pub mod adas;
pub mod body;
pub mod cabin;
pub mod exterior;
pub mod powertrain;

use crate::{DdsData, Piccoloable};
use dust_dds::{
    dds_async::domain_participant_factory::DomainParticipantFactoryAsync,
    infrastructure::{error::DdsResult, qos::QosKind, status::NO_STATUS},
    subscription::sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
    topic_definition::type_support::{DdsDeserialize, DdsHasKey, DdsKey, DdsTypeXml},
};
use tokio::sync::mpsc::Sender;

pub async fn receive_dds<
    T: Piccoloable + DdsKey + DdsHasKey + DdsTypeXml + for<'a> DdsDeserialize<'a> + 'static,
>(
    tx: Sender<DdsData>,
) {
    let (topic_name, type_name, domain_id) = (T::topic_name(), T::type_name(), 0);
    let participant_factory = DomainParticipantFactoryAsync::new();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, None, NO_STATUS)
        .await
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, None, NO_STATUS)
        .await
        .unwrap();

    let topic = participant
        .create_topic::<T>(&topic_name, &type_name, QosKind::Default, None, NO_STATUS)
        .await
        .unwrap();
    let reader = subscriber
        .create_datareader::<T>(&topic, QosKind::Default, None, NO_STATUS)
        .await
        .unwrap();

    println!("make loop - {topic_name}");
    loop {
        if let Ok(data_samples) = reader
            .take(50, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
            .await
        {
            let data: DdsResult<T> = data_samples[0].data();
            if data.is_err() {
                continue;
            }
            let msg = data.unwrap().to_piccolo_dds_data();
            println!("Received: {:?}", msg);
            let _ = tx.send(msg).await;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
