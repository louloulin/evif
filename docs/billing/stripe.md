# Stripe Payment Integration

## Overview

This document describes the Stripe payment integration for EVIF's Plugin Marketplace and subscription billing.

## Configuration

### Environment Variables

```bash
# Stripe Configuration
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PUBLISHABLE_KEY=pk_test_...
```

### Configuration File

```toml
[billing.stripe]
enabled = true
secret_key = "${STRIPE_SECRET_KEY}"
webhook_secret = "${STRIPE_WEBHOOK_SECRET}"
publishable_key = "${STRIPE_PUBLISHABLE_KEY}"

# Product Configuration
[billing.stripe.products]
pro = "prod_pro_monthly"
team = "prod_team_monthly"
enterprise = "prod_enterprise_monthly"

# Plugin Pricing
[billing.stripe.plugin_pricing]
min_price_cents = 100
max_price_cents = 999999
```

## Pricing Tiers

### Subscription Plans

| Plan | Price | Features |
|------|-------|----------|
| Free | $0 | 1,000 API calls/day, 100MB storage |
| Pro | $29/mo | 100,000 API calls/day, 10GB storage |
| Team | $99/mo | 1,000,000 API calls/day, 100GB storage |
| Enterprise | $499/mo | Unlimited, dedicated support |

### Plugin Marketplace Pricing

| Model | Description | Revenue Share |
|-------|-------------|----------------|
| Free | Free to use | N/A |
| Paid | One-time purchase | Developer 70%, EVIF 30% |
| Subscription | Monthly recurring | Developer 70%, EVIF 30% |
| Freenium | Free tier + paid upgrades | Developer 70%, EVIF 30% |

## API Integration

### Create Subscription

```rust
async fn create_subscription(
    tenant_id: &str,
    plan: PricingPlan,
) -> Result<Subscription> {
    // 1. Get or create Stripe customer
    let customer = get_or_create_customer(tenant_id).await?;
    
    // 2. Get price ID for plan
    let price_id = get_price_id(plan)?;
    
    // 3. Create subscription
    let subscription = stripe.subscriptions().create(&[
        ("customer", &customer.id.into()),
        ("items", &vec![("price", &price_id.into())].into()),
    ]).await?;
    
    // 4. Store subscription metadata
    store_subscription(tenant_id, &subscription).await?;
    
    Ok(subscription)
}
```

### Create Plugin Payment

```rust
async fn create_plugin_payment(
    plugin_id: &str,
    buyer_id: &str,
) -> Result<PaymentIntent> {
    // 1. Get plugin price
    let plugin = get_plugin(plugin_id).await?;
    let amount = calculate_amount(&plugin.pricing)?;
    
    // 2. Create payment intent
    let intent = stripe.payment_intents().create(&[
        ("amount", &amount.into()),
        ("currency", &"usd".into()),
        ("metadata", &json!({
            "plugin_id": plugin_id,
            "buyer_id": buyer_id,
        }).into()),
    ]).await?;
    
    Ok(intent)
}
```

### Webhook Handling

```rust
async fn handle_webhook(
    payload: &[u8],
    signature: &str,
) -> Result<()> {
    // 1. Verify webhook signature
    let event = stripe.webhooks().construct_event(
        payload,
        signature,
        WEBHOOK_SECRET,
    )?;
    
    // 2. Handle event
    match event.type_ {
        "invoice.paid" => {
            handle_invoice_paid(&event.data).await?;
        }
        "customer.subscription.updated" => {
            handle_subscription_updated(&event.data).await?;
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(&event.data).await?;
        }
        "payment_intent.succeeded" => {
            handle_payment_succeeded(&event.data).await?;
        }
        _ => {}
    }
    
    Ok(())
}
```

## Marketplace Revenue Distribution

### Developer Payout Flow

```
1. Buyer purchases plugin → Payment received
2. EVIF takes 30% platform fee
3. Developer receives 70%
4. Monthly payout via Stripe Connect
```

### Stripe Connect Setup

```rust
async fn create_developer_connect_account(
    developer_id: &str,
) -> Result<Account> {
    let account = stripe.accounts().create(&[
        ("type", &"express".into()),
        ("email", &developer_email.into()),
        ("metadata", &json!({
            "developer_id": developer_id,
        }).into()),
    ]).await?;
    
    // Store account ID for future payouts
    store_connect_account(developer_id, &account.id).await?;
    
    Ok(account)
}
```

### Calculate Payout

```rust
async fn calculate_payout(
    payment_amount_cents: u32,
) -> (u32, u32) {
    let evif_fee = payment_amount_cents * 30 / 100;  // 30%
    let developer_amount = payment_amount_cents - evif_fee;  // 70%
    (developer_amount, evif_fee)
}
```

## Billing Portal

### Customer Portal Integration

```rust
async fn create_billing_portal_session(
    customer_id: &str,
    return_url: &str,
) -> Result<String> {
    let session = stripe.billing_portal().sessions().create(&[
        ("customer", &customer_id.into()),
        ("return_url", &return_url.into()),
    ]).await?;
    
    Ok(session.url)
}
```

### Embed Stripe Elements

```html
<!-- Payment Form -->
<div id="payment-element"></div>
<button id="submit">Subscribe</button>

<script>
const stripe = Stripe('pk_test_...');
const elements = stripe.elements();
const paymentElement = elements.create('payment');
paymentElement.mount('#payment-element');

document.getElementById('submit').onclick = async () => {
  const { error } = await stripe.confirmPayment({
    elements,
    confirmParams: {
      return_url: 'https://evif.io/subscribe/success',
    },
  });
};
</script>
```

## Usage Quotas

### Track API Usage

```rust
async fn track_usage(
    tenant_id: &str,
    endpoint: &str,
) -> Result<()> {
    // Increment usage counter
    redis.incr(&format!("usage:{tenant_id}:daily")).await?;
    
    // Check quota
    let usage = get_current_usage(tenant_id).await?;
    let quota = get_quota(tenant_id).await?;
    
    if usage > quota {
        return Err(Error::QuotaExceeded);
    }
    
    Ok(())
}
```

### Quota Alerts

```rust
async fn check_quota_alerts(
    tenant_id: &str,
) -> Result<()> {
    let usage_pct = get_usage_percentage(tenant_id).await?;
    
    match usage_pct {
        80..=99 => {
            // Send warning webhook
            send_webhook(tenant_id, "quota_80_percent").await?;
        }
        100 => {
            // Upgrade required
            send_webhook(tenant_id, "quota_100_percent").await?;
        }
        _ => {}
    }
    
    Ok(())
}
```

## Error Handling

### Payment Failures

```rust
match error {
    stripe::Error::CardError { code, .. } => {
        match code.as_str() {
            "insufficient_funds" => {
                // Show retry with different payment method
            }
            "expired_card" => {
                // Prompt to update card
            }
            _ => {
                // Generic decline message
            }
        }
    }
    _ => {
        // Log and show generic error
    }
}
```

### Subscription Issues

| Issue | Resolution |
|-------|------------|
| Payment failed | Retry 3 times, then suspend |
| Card expired | Send email, prompt update |
| Subscription cancelled | Grace period 7 days |
| Refund requested | Process within 30 days |

## Testing

### Test Cards

```bash
# Success
STRIPE_TEST_CARD=4242424242424242

# Decline
STRIPE_TEST_CARD=4000000000000002

# Insufficient funds
STRIPE_TEST_CARD=4000000000009995
```

### Test Webhooks

```bash
stripe listen --forward-to localhost:8080/webhooks/stripe
```

## Security

- All Stripe API calls use TLS
- Webhook signatures verified on every event
- PCI compliance via Stripe Elements
- No card data stored on EVIF servers
- Rate limiting on payment endpoints

## Monitoring

### Metrics to Track

| Metric | Description |
|--------|-------------|
| `stripe_subscriptions_total` | Total active subscriptions |
| `stripe_payments_total` | Payment attempts |
| `stripe_payments_failed_total` | Failed payments |
| `stripe_revenue_cents` | Revenue collected |
| `stripe_payouts_pending` | Pending developer payouts |

## Migration Guide

### From Manual Billing

1. Export existing customer data
2. Create Stripe customers for each
3. Create subscriptions matching existing plans
4. Migrate usage data to Redis
5. Test end-to-end payment flow
6. Switch traffic to Stripe

## Support

- Stripe Dashboard: https://dashboard.stripe.com
- Documentation: https://stripe.com/docs
- Support: support@evif.io
