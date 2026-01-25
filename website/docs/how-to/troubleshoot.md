---
sidebar_position: 5
---

# Troubleshoot Common Issues

## Connection Issues

### "Connection refused" to upstream

**Symptom:** Requests fail with upstream connection errors.

**Check:**

```bash
# Is the upstream reachable?
curl http://your-upstream:3000/health

# Check ThoughtGate logs
docker logs thoughtgate 2>&1 | grep -i upstream
```

**Fix:**
- Verify `THOUGHTGATE_UPSTREAM_URL` is correct
- In Kubernetes, ensure the service DNS is resolvable
- Check network policies aren't blocking traffic

### Requests timeout

**Symptom:** Requests hang and eventually timeout.

**Check:**

```bash
# Check upstream latency
time curl http://your-upstream:3000/health
```

**Fix:**
- Increase `THOUGHTGATE_REQUEST_TIMEOUT_SECS`
- Check upstream server performance
- For approval requests, check Slack connectivity

## Slack Integration

### Approval messages not appearing

**Symptom:** Requests block but no Slack message appears.

**Check:**

```bash
# Verify token is set
echo $THOUGHTGATE_SLACK_BOT_TOKEN | head -c 10

# Check ThoughtGate logs for Slack errors
docker logs thoughtgate 2>&1 | grep -i slack
```

**Fix:**
- Verify the bot token has required scopes: `chat:write`, `reactions:read`, `channels:history`
- Ensure the bot is invited to the channel: `/invite @YourBot`
- Check the channel name includes `#`

### Reactions not detected

**Symptom:** Messages appear but 👍/👎 reactions are ignored.

**Check:**
- Verify `reactions:read` scope is granted
- Check the bot can read channel history (`channels:history`)

**Fix:**
- Re-add the bot to the channel
- Verify the reaction is on the correct message (not a thread reply)

## Policy Issues

### All requests denied

**Symptom:** Every request returns "Policy denied".

**Check:**

```bash
# Validate Cedar syntax
cedar validate --policies policy.cedar

# Check ThoughtGate logs
docker logs thoughtgate 2>&1 | grep -i policy
```

**Fix:**
- Ensure you have a `permit` rule that matches
- Check for syntax errors in the policy file
- Verify the policy file path is correct

### Policy not reloading

**Symptom:** Changes to policy file aren't applied.

**Check:**
- File is readable by ThoughtGate
- No syntax errors in the updated policy

**Fix:**
- Check logs for "Policy reloaded" or error messages
- Restart ThoughtGate if hot reload fails

## Performance Issues

### High latency

**Symptom:** Proxy adds significant latency.

**Check:**

```bash
# Measure overhead
time curl -X POST http://localhost:8080 -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

**Fix:**
- Enable connection pooling (default)
- Check if policy evaluation is slow (complex policies)
- Ensure adequate CPU resources

### Memory growing

**Symptom:** Memory usage increases over time.

**Check:**

```bash
# Monitor memory
docker stats thoughtgate
```

**Fix:**
- Check for stale pending approvals (timeout them)
- Verify no request body buffering issues
- Report if consistently growing (may be a bug)

## Health Check Failures

### Liveness probe failing

**Symptom:** Kubernetes keeps restarting the pod.

**Check:**

```bash
curl http://localhost:8081/health
```

**Fix:**
- Increase `initialDelaySeconds`
- Check if the process is actually crashing (logs)

### Readiness probe failing

**Symptom:** Pod not receiving traffic.

**Check:**

```bash
curl http://localhost:8081/ready
```

**Fix:**
- Verify upstream is reachable
- Check policy file is loaded

## Getting Help

If you're still stuck:

1. Check the [GitHub Issues](https://github.com/thoughtgate/thoughtgate/issues)
2. Open a new issue with:
   - ThoughtGate version
   - Configuration (redact secrets)
   - Relevant logs
   - Steps to reproduce
