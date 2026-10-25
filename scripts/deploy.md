# AWS commands

The examples use PowerShell and the AWS CLI v2, with region `ap-southeast-2`
and account `<ACCOUNT_ID>`. On this PC, `aws.exe` isn't on `PATH`. Either use
the full path, or set an alias for the session:

```powershell
Set-Alias aws "C:\Program Files\Amazon\AWSCLIV2\aws.exe"
```

The sections are:

1. **Commands Claude ran** on 2026-10-25. All of them were read-only.
2. **Fix the RDS security group.** Not run yet.
3. **Full deploy from scratch.** Reconstructed from the live config. I didn't
   run these and they weren't captured from the original setup, so check each
   one before using it.

---

## 1. Commands Claude ran (read-only)

### Who am I, and which region

```powershell
aws sts get-caller-identity
aws configure get region
```

### ECS service and task status

```powershell
aws ecs list-clusters
aws ecs list-services --cluster aws-app --query "serviceArns" --output text

aws ecs describe-services --cluster aws-app --services aws-app `
  --query "services[].{name:serviceName,status:status,desired:desiredCount,running:runningCount,pending:pendingCount,rollout:deployments[0].rolloutState,taskDef:taskDefinition,lbs:loadBalancers,events:events[0:5].[createdAt,message]}" `
  --output json

aws ecs describe-services --cluster aws-app --services aws-app `
  --query "services[0].{launchType:launchType,platformVersion:platformVersion,net:networkConfiguration,enableExecuteCommand:enableExecuteCommand}" `
  --output json
```

### Task status, public IP, and /health

The task ID changes on every deployment. Look up the current one with
`aws ecs list-tasks --cluster aws-app --service-name aws-app`.

```powershell
$task = (aws ecs list-tasks --cluster aws-app --service-name aws-app --query "taskArns[0]" --output text)
$t = aws ecs describe-tasks --cluster aws-app --tasks $task --query "tasks[0]" --output json | ConvertFrom-Json
"lastStatus=$($t.lastStatus) health=$($t.healthStatus) started=$($t.startedAt)"
$t.containers | ForEach-Object { "container $($_.name): $($_.lastStatus) exit=$($_.exitCode)" }

$eni = ($t.attachments[0].details | Where-Object name -eq 'networkInterfaceId').value
$ip  = aws ec2 describe-network-interfaces --network-interface-ids $eni `
         --query "NetworkInterfaces[0].Association.PublicIp" --output text
"publicIp=$ip"

Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "http://${ip}:8080/health"
```

`/health` doesn't touch the database. To prove the database is reachable,
call a route that queries it. A login with an unknown email should return
**401** quickly. A timeout or a 500 means the database can't be reached.

```powershell
Invoke-WebRequest -UseBasicParsing -TimeoutSec 20 -Method Post -ContentType 'application/json' `
  -Body '{"email":"nobody@example.invalid","password":"not-a-real-password"}' `
  "http://${ip}:8080/api/v1/auth/login"
```

### Task definition (image, env, secrets, roles, logging)

```powershell
aws ecs describe-task-definition --task-definition aws-app:2 `
  --query "taskDefinition.{family:family,rev:revision,cpu:cpu,mem:memory,net:networkMode,compat:requiresCompatibilities,platform:runtimePlatform,exec:executionRoleArn,task:taskRoleArn,containers:containerDefinitions[].{name:name,image:image,ports:portMappings,env:environment,secrets:secrets,log:logConfiguration,hc:healthCheck}}" `
  --output json
```

### CloudWatch logs (proof that migrations ran)

```powershell
aws logs describe-log-streams --log-group-name /ecs/aws-app --order-by LastEventTime --descending --max-items 5 `
  --query "logStreams[].{name:logStreamName,first:firstEventTimestamp,last:lastEventTimestamp}" --output json

aws logs get-log-events --log-group-name /ecs/aws-app `
  --log-stream-name app/aws-app/<task-id> --start-from-head --limit 40 `
  --query "events[].[timestamp,message]" --output text

# Or just search for the line:
aws logs filter-log-events --log-group-name /ecs/aws-app --filter-pattern '"migrations applied"' `
  --query "events[].[logStreamName,message]" --output text
```

### IAM roles

```powershell
aws iam list-attached-role-policies --role-name aws-app-ecs-execution
aws iam list-role-policies --role-name aws-app-ecs-execution
aws iam get-role-policy --role-name aws-app-ecs-execution --policy-name read-aws-app-ssm --query "PolicyDocument"
aws iam get-role --role-name aws-app-ecs-execution --query "Role.AssumeRolePolicyDocument"

aws iam list-instance-profiles --query "InstanceProfiles[?InstanceProfileName=='ec2-ssm-role'].Roles[].RoleName" --output text
aws iam list-attached-role-policies --role-name ec2-ssm-role --query "AttachedPolicies[].PolicyName" --output text
aws iam list-role-policies --role-name ec2-ssm-role --query "PolicyNames" --output text
```

### RDS, bastion, network, ECR, SSM parameters

```powershell
aws rds describe-db-instances `
  --query "DBInstances[].{id:DBInstanceIdentifier,engine:Engine,ver:EngineVersion,class:DBInstanceClass,public:PubliclyAccessible,endpoint:Endpoint.Address,port:Endpoint.Port,sgs:VpcSecurityGroups[].VpcSecurityGroupId,subnetGroup:DBSubnetGroup.DBSubnetGroupName,vpc:DBSubnetGroup.VpcId,encrypted:StorageEncrypted,multiAZ:MultiAZ,db:DBName}" `
  --output json

aws ec2 describe-instances --filters "Name=tag:Name,Values=db-bastion" `
  --query "Reservations[].Instances[].{id:InstanceId,state:State.Name,type:InstanceType,arch:Architecture,subnet:SubnetId,publicIp:PublicIpAddress,profile:IamInstanceProfile.Arn,sgs:SecurityGroups,key:KeyName}" `
  --output json

aws ec2 describe-security-groups --group-ids <ECS_SG_ID> <DB_SG_ID> <BASTION_SG_ID> `
  --query "SecurityGroups[].{id:GroupId,name:GroupName,in:IpPermissions[].{proto:IpProtocol,from:FromPort,to:ToPort,cidr:IpRanges[].CidrIp,sg:UserIdGroupPairs[].GroupId},out:IpPermissionsEgress[].{proto:IpProtocol,from:FromPort,to:ToPort,cidr:IpRanges[].CidrIp,sg:UserIdGroupPairs[].GroupId}}" `
  --output json

aws ec2 describe-security-group-rules --filters Name=group-id,Values=<DB_SG_ID> `
  --query "SecurityGroupRules[?!IsEgress].{id:SecurityGroupRuleId,desc:Description,src:ReferencedGroupInfo.GroupId}" --output json

aws ec2 describe-subnets --subnet-ids <SUBNET_ID_2A> <SUBNET_ID_2B> <SUBNET_ID_2C> `
  --query "Subnets[].{id:SubnetId,az:AvailabilityZone,cidr:CidrBlock,publicIpOnLaunch:MapPublicIpOnLaunch,default:DefaultForAz}" --output json
aws ec2 describe-vpcs --vpc-ids <VPC_ID> --query "Vpcs[0].{cidr:CidrBlock,default:IsDefault}" --output json

aws ecr describe-repositories `
  --query "repositories[].{name:repositoryName,uri:repositoryUri,scan:imageScanningConfiguration.scanOnPush,mutability:imageTagMutability}" --output json
aws ecr describe-images --repository-name aws_app `
  --query "sort_by(imageDetails,&imagePushedAt)[-3:].{tags:imageTags,pushed:imagePushedAt,mb:imageSizeInBytes}" --output json

aws logs describe-log-groups --log-group-name-prefix /ecs/aws-app --query "logGroups[].{name:logGroupName,retention:retentionInDays}" --output json

# Metadata only. This does not print the secret values.
aws ssm describe-parameters --parameter-filters "Key=Name,Option=BeginsWith,Values=/food-ordering/" `
  --query "Parameters[].{name:Name,type:Type,key:KeyId,modified:LastModifiedDate}" --output json
```

### Who changed the RDS security group (CloudTrail)

```powershell
aws cloudtrail lookup-events --lookup-attributes AttributeKey=ResourceName,AttributeValue=<DB_SG_ID> `
  --max-results 20 --query "Events[].[EventTime,EventName,Username]" --output text

$e = aws cloudtrail lookup-events --lookup-attributes AttributeKey=EventName,AttributeValue=ModifySecurityGroupRules --max-results 5 --output json | ConvertFrom-Json
foreach ($ev in $e.Events) { $ct = $ev.CloudTrailEvent | ConvertFrom-Json; "$($ev.EventTime) $($ct.userIdentity.arn)"; $ct.requestParameters | ConvertTo-Json -Depth 10 }
```

---

## 2. Fix the RDS security group (not run yet)

Rule `<DB_SG_RULE_ID>` on `aws-app-db-sg` was edited from "5432 from
`aws-app-ecs`" to "5432 from `db-bastion-sg`", so the app can no longer reach
RDS. Both sources need access. Point the existing rule back at the ECS SG, and
add a separate rule for the bastion:

```powershell
aws ec2 modify-security-group-rules --group-id <DB_SG_ID> `
  --security-group-rules 'SecurityGroupRuleId=<DB_SG_RULE_ID>,SecurityGroupRule={IpProtocol=tcp,FromPort=5432,ToPort=5432,ReferencedGroupId=<ECS_SG_ID>,Description=aws-app-ecs-tasks}'

aws ec2 authorize-security-group-ingress --group-id <DB_SG_ID> `
  --ip-permissions 'IpProtocol=tcp,FromPort=5432,ToPort=5432,UserIdGroupPairs=[{GroupId=<BASTION_SG_ID>,Description=db-bastion}]'
```

Then re-run the login probe from section 1. The running task's connection pool
reconnects on its own once the rule allows traffic again.

---

## 3. Full deploy from scratch (reconstructed, not run)

These steps rebuild the current setup. Placeholders look like `<...>`.

### 3.1 Build and push the image to ECR

```powershell
aws ecr create-repository --repository-name aws_app --image-scanning-configuration scanOnPush=true

aws ecr get-login-password | docker login --username AWS --password-stdin <ACCOUNT_ID>.dkr.ecr.ap-southeast-2.amazonaws.com

$tag = Get-Date -Format 'yyyyMMdd-HHmmss'
docker build -t aws_app:$tag .          # for arm64: docker buildx build --platform linux/arm64 ...
docker tag aws_app:$tag <ACCOUNT_ID>.dkr.ecr.ap-southeast-2.amazonaws.com/aws_app:$tag
docker push <ACCOUNT_ID>.dkr.ecr.ap-southeast-2.amazonaws.com/aws_app:$tag
```

### 3.2 Security groups (default VPC `<VPC_ID>`)

```powershell
$myIp = (Invoke-RestMethod https://checkip.amazonaws.com).Trim()

$ecsSg     = aws ec2 create-security-group --group-name aws-app-ecs   --description "aws-app Fargate tasks" --vpc-id <VPC_ID> --query GroupId --output text
$dbSg      = aws ec2 create-security-group --group-name aws-app-db-sg --description "aws-app RDS"           --vpc-id <VPC_ID> --query GroupId --output text
$bastionSg = aws ec2 create-security-group --group-name db-bastion-sg --description "SSM-only DB bastion"   --vpc-id <VPC_ID> --query GroupId --output text

aws ec2 authorize-security-group-ingress --group-id $ecsSg `
  --ip-permissions "IpProtocol=tcp,FromPort=8080,ToPort=8080,IpRanges=[{CidrIp=$myIp/32,Description=owner-ip}]"
aws ec2 authorize-security-group-ingress --group-id $dbSg `
  --ip-permissions "IpProtocol=tcp,FromPort=5432,ToPort=5432,UserIdGroupPairs=[{GroupId=$ecsSg,Description=aws-app-ecs-tasks}]"
aws ec2 authorize-security-group-ingress --group-id $dbSg `
  --ip-permissions "IpProtocol=tcp,FromPort=5432,ToPort=5432,UserIdGroupPairs=[{GroupId=$bastionSg,Description=db-bastion}]"
# db-bastion-sg gets no inbound rules.
```

### 3.3 RDS Postgres 18

```powershell
aws rds create-db-instance --db-instance-identifier aws-app-db `
  --engine postgres --engine-version 18.3 --db-instance-class db.t4g.micro `
  --allocated-storage 20 --storage-encrypted --no-publicly-accessible `
  --db-name aws_app --master-username <master-user> --master-user-password '<strong-password>' `
  --vpc-security-group-ids $dbSg

aws rds wait db-instance-available --db-instance-identifier aws-app-db
```

### 3.4 Secrets in SSM Parameter Store

```powershell
aws ssm put-parameter --name /food-ordering/DATABASE_URL --type SecureString `
  --value 'postgres://<user>:<password>@aws-app-db.<RDS_ID>.ap-southeast-2.rds.amazonaws.com:5432/aws_app'
aws ssm put-parameter --name /food-ordering/JWT_SECRET --type SecureString --value '<openssl rand -base64 48>'
# Add --overwrite to change an existing value, then force a new ECS deployment.
```

### 3.5 Task execution role

Save the trust policy as `ecs-trust.json`:

```json
{ "Version": "2012-10-17", "Statement": [ { "Effect": "Allow", "Principal": { "Service": "ecs-tasks.amazonaws.com" }, "Action": "sts:AssumeRole" } ] }
```

Save the SSM read policy as `read-aws-app-ssm.json`:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    { "Effect": "Allow", "Action": ["ssm:GetParameters"], "Resource": "arn:aws:ssm:ap-southeast-2:<ACCOUNT_ID>:parameter/food-ordering/*" },
    { "Effect": "Allow", "Action": ["kms:Decrypt"], "Resource": "*", "Condition": { "StringEquals": { "kms:ViaService": "ssm.ap-southeast-2.amazonaws.com" } } }
  ]
}
```

Then create the role and attach both policies:

```powershell
aws iam create-role --role-name aws-app-ecs-execution --assume-role-policy-document file://ecs-trust.json
aws iam attach-role-policy --role-name aws-app-ecs-execution --policy-arn arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy
aws iam put-role-policy --role-name aws-app-ecs-execution --policy-name read-aws-app-ssm --policy-document file://read-aws-app-ssm.json
```

There's no task role, because the app doesn't call any AWS API.

### 3.6 Logs, cluster, task definition, service

```powershell
aws logs create-log-group --log-group-name /ecs/aws-app
aws logs put-retention-policy --log-group-name /ecs/aws-app --retention-in-days 14

aws ecs create-cluster --cluster-name aws-app
```

Save the task definition as `taskdef.json`, setting the image tag:

```json
{
  "family": "aws-app",
  "requiresCompatibilities": ["FARGATE"],
  "networkMode": "awsvpc",
  "cpu": "256",
  "memory": "512",
  "runtimePlatform": { "cpuArchitecture": "X86_64", "operatingSystemFamily": "LINUX" },
  "executionRoleArn": "arn:aws:iam::<ACCOUNT_ID>:role/aws-app-ecs-execution",
  "containerDefinitions": [
    {
      "name": "aws-app",
      "image": "<ACCOUNT_ID>.dkr.ecr.ap-southeast-2.amazonaws.com/aws_app:<tag>",
      "essential": true,
      "portMappings": [ { "containerPort": 8080, "hostPort": 8080, "protocol": "tcp" } ],
      "environment": [
        { "name": "APP_HOST", "value": "0.0.0.0" },
        { "name": "APP_PORT", "value": "8080" },
        { "name": "RUST_LOG", "value": "info" },
        { "name": "JWT_TTL_MINUTES", "value": "60" },
        { "name": "DATABASE_MAX_CONNECTIONS", "value": "5" }
      ],
      "secrets": [
        { "name": "DATABASE_URL", "valueFrom": "arn:aws:ssm:ap-southeast-2:<ACCOUNT_ID>:parameter/food-ordering/DATABASE_URL" },
        { "name": "JWT_SECRET",   "valueFrom": "arn:aws:ssm:ap-southeast-2:<ACCOUNT_ID>:parameter/food-ordering/JWT_SECRET" }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": { "awslogs-group": "/ecs/aws-app", "awslogs-region": "ap-southeast-2", "awslogs-stream-prefix": "app" }
      }
    }
  ]
}
```

Register it and create the service:

```powershell
aws ecs register-task-definition --cli-input-json file://taskdef.json

aws ecs create-service --cluster aws-app --service-name aws-app --task-definition aws-app `
  --desired-count 1 --launch-type FARGATE `
  --network-configuration "awsvpcConfiguration={subnets=[<SUBNET_ID_2A>,<SUBNET_ID_2B>,<SUBNET_ID_2C>],securityGroups=[$ecsSg],assignPublicIp=ENABLED}"

aws ecs wait services-stable --cluster aws-app --services aws-app
```

### 3.7 Redeploy a new image

Build and push a new tag (3.1), change `image` in `taskdef.json`, and run:

```powershell
aws ecs register-task-definition --cli-input-json file://taskdef.json
aws ecs update-service --cluster aws-app --service aws-app --task-definition aws-app
aws ecs wait services-stable --cluster aws-app --services aws-app
```

If you only changed a secret, the task definition stays the same:

```powershell
aws ecs update-service --cluster aws-app --service aws-app --force-new-deployment
```

### 3.8 Bastion for DB access (SSM only)

```powershell
aws iam create-role --role-name ec2-ssm-role --assume-role-policy-document '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"ec2.amazonaws.com"},"Action":"sts:AssumeRole"}]}'
aws iam attach-role-policy --role-name ec2-ssm-role --policy-arn arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore
aws iam create-instance-profile --instance-profile-name ec2-ssm-role
aws iam add-role-to-instance-profile --instance-profile-name ec2-ssm-role --role-name ec2-ssm-role

$ami = aws ssm get-parameter --name /aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64 --query Parameter.Value --output text
aws ec2 run-instances --image-id $ami --instance-type t3.micro --subnet-id <SUBNET_ID_2B> `
  --security-group-ids $bastionSg --iam-instance-profile Name=ec2-ssm-role `
  --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=db-bastion}]'
# No --key-name: there's no SSH key, and access is through SSM only.
```

Open the tunnel. Leave it running, and connect to `localhost:5433` from
another terminal:

```powershell
aws ssm start-session --target <BASTION_INSTANCE_ID> `
  --document-name AWS-StartPortForwardingSessionToRemoteHost `
  --parameters '{"host":["aws-app-db.<RDS_ID>.ap-southeast-2.rds.amazonaws.com"],"portNumber":["5432"],"localPortNumber":["5433"]}'
```
