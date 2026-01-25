---
sidebar_position: 4
---

# Deploy to Kubernetes

ThoughtGate is designed to run as a sidecar container alongside your AI agent pods.

## Sidecar Deployment

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: my-agent
spec:
  containers:
    # Your AI agent
    - name: agent
      image: my-agent:latest
      env:
        - name: MCP_SERVER_URL
          value: "http://localhost:8080"  # Points to ThoughtGate

    # ThoughtGate sidecar
    - name: thoughtgate
      image: ghcr.io/thoughtgate/thoughtgate:latest
      ports:
        - containerPort: 8080
          name: proxy
        - containerPort: 8081
          name: admin
      env:
        - name: THOUGHTGATE_UPSTREAM_URL
          value: "http://mcp-server:3000"
        - name: THOUGHTGATE_SLACK_BOT_TOKEN
          valueFrom:
            secretKeyRef:
              name: thoughtgate-secrets
              key: slack-token
        - name: THOUGHTGATE_SLACK_CHANNEL
          value: "#approvals"
      livenessProbe:
        httpGet:
          path: /health
          port: admin
        initialDelaySeconds: 5
        periodSeconds: 10
      readinessProbe:
        httpGet:
          path: /ready
          port: admin
        initialDelaySeconds: 5
        periodSeconds: 5
      resources:
        requests:
          memory: "20Mi"
          cpu: "50m"
        limits:
          memory: "100Mi"
          cpu: "200m"
```

## Create the Secret

```bash
kubectl create secret generic thoughtgate-secrets \
  --from-literal=slack-token=xoxb-your-token
```

## ConfigMap for Policies

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: thoughtgate-policy
data:
  policy.cedar: |
    permit(
        principal,
        action == Action::"tools/list",
        resource
    );

    permit(
        principal,
        action == Action::"tools/call",
        resource
    ) when {
        resource.tool_name in ["delete_user", "drop_table"]
    } advice {
        "require_approval": true
    };

    permit(
        principal,
        action == Action::"tools/call",
        resource
    );
```

Mount the ConfigMap:

```yaml
containers:
  - name: thoughtgate
    # ...
    env:
      - name: THOUGHTGATE_CEDAR_POLICY_PATH
        value: "/etc/thoughtgate/policy.cedar"
    volumeMounts:
      - name: policy
        mountPath: /etc/thoughtgate
        readOnly: true
volumes:
  - name: policy
    configMap:
      name: thoughtgate-policy
```

## Health Checks

ThoughtGate exposes health endpoints on the admin port:

| Endpoint | Purpose | Use Case |
|----------|---------|----------|
| `/health` | Liveness | Process is running |
| `/ready` | Readiness | Ready to accept traffic |

## Resource Recommendations

| Workload | Memory Request | Memory Limit | CPU Request | CPU Limit |
|----------|----------------|--------------|-------------|-----------|
| Light | 20Mi | 50Mi | 25m | 100m |
| Medium | 50Mi | 100Mi | 50m | 200m |
| Heavy | 100Mi | 200Mi | 100m | 500m |

## Monitoring

Expose Prometheus metrics:

```yaml
annotations:
  prometheus.io/scrape: "true"
  prometheus.io/port: "8081"
  prometheus.io/path: "/metrics"
```

## Next Steps

- Review the [Configuration Reference](/docs/reference/configuration)
- Learn about [traffic tiers](/docs/explanation/traffic-tiers)
