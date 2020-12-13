use async_trait::async_trait;
use rusoto_ec2::{Ec2, Ec2Client};

use rusoto_ec2::DescribeInstancesRequest;
use crate::error::MetricsNotifierError;

struct Ec2InstanceClient {
    client: Ec2Client,
}

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

        let result = self.client.describe_instances(request).await;
        Ok(Vec::new())
    }
}


#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_describe_all_instances() {

    }
}