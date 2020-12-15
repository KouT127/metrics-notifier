use async_trait::async_trait;
use rusoto_ec2::{Ec2, Ec2Client};

use rusoto_ec2::DescribeInstancesRequest;
use crate::error::MetricsNotifierError;

struct Ec2InstanceClient {
    client: Ec2Client,
}

#[derive(Debug, PartialEq)]
struct MachineInstance {
    instance_id: String
}

#[async_trait]
trait Describe {
    async fn describe_all_instances(&self) -> Result<Vec<MachineInstance>, MetricsNotifierError>;
}

#[async_trait]
impl Describe for Ec2InstanceClient {
    async fn describe_all_instances(&self) -> Result<Vec<MachineInstance>, MetricsNotifierError> {
        let request = DescribeInstancesRequest {
            max_results: Some(20),
            ..DescribeInstancesRequest::default()
        };

        let result = self.client.describe_instances(request).await.map_err(|error| MetricsNotifierError::DescribeInstancesError(error))?;

        let mut machine_instances = Vec::<MachineInstance>::new();
        for reservation in result.reservations.ok_or(MetricsNotifierError::NoneValue)? {
            for instance in reservation.instances.ok_or(MetricsNotifierError::NoneValue)? {
                machine_instances.push(MachineInstance {
                    instance_id: instance.instance_id.ok_or(MetricsNotifierError::NoneValue)?,
                })
            }
        }
        Ok(machine_instances)
    }
}

impl Ec2InstanceClient {
    fn new_with_client(client: Ec2Client) -> Self {
        Ec2InstanceClient {
            client
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::ec2_instance_client::{Ec2InstanceClient, Describe, MachineInstance};
    use rusoto_mock::{MockCredentialsProvider, MockRequestDispatcher, MockResponseReader, ReadMockResponse};
    use rusoto_ec2::Ec2Client;

    #[tokio::test]
    async fn test_describe_all_instances() {
        let mock = Ec2Client::new_with(
            MockRequestDispatcher::default().with_body(&*MockResponseReader::read_response(
                "test_resources/valid",
                "describe_instances.xml",
            )),
            MockCredentialsProvider,
            Default::default(),
        );

        let client = Ec2InstanceClient::new_with_client(mock);
        let result = client.describe_all_instances().await;

        assert_eq!(
            result.unwrap(),
            [MachineInstance {
                instance_id: "i-1234567890abcdef0".to_string()
            }]
        );
    }
}