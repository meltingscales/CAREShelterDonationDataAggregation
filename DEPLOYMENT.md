# Deployment Guide

This guide covers deploying the C.A.R.E. Shelter Donation Data Aggregation tool to Google Cloud Platform.

## Prerequisites

1. **Google Cloud SDK** - Install `gcloud` CLI tool
2. **Docker** - Install Docker for building images
3. **GCP Project** - Create a GCP project
4. **Permissions** - Enable Cloud Run API and have appropriate IAM permissions

## Configuration

Set your GCP project details as environment variables:

```bash
export GCP_PROJECT="your-project-id"
export GCP_REGION="us-central1"  # or your preferred region
export DOMAIN_NAME="meltingscales-donorsnap-care.yourdomain.com"
```

## Quick Deployment

Deploy everything in one command:

```bash
just gcp-deploy-all
```

This will:
1. Build the Docker image
2. Push it to Google Container Registry
3. Deploy to Cloud Run

## Step-by-Step Deployment

### 1. Build and Push Docker Image

```bash
# Build and push to GCR
just gcp-push

# Or with a specific tag
just gcp-push-tag v1.0.0
```

### 2. Deploy to Cloud Run

```bash
# Deploy latest version
just gcp-deploy

# Or deploy a specific tag
just gcp-deploy-tag v1.0.0
```

### 3. Get Service URL

```bash
just gcp-url
```

This will output something like: `https://care-shelter-donation-aggregation-xyz123-uc.a.run.app`

## Custom Domain Setup

### Step 1: Add Domain Mapping

```bash
# Using the DOMAIN_NAME environment variable
just gcp-domain-map

# Or specify a domain directly
just gcp-domain-map donate.yoursite.com
```

### Step 2: Get DNS Records

After creating the domain mapping, get the DNS records you need to configure:

```bash
just gcp-domain-records donate.yoursite.com
```

This will show you the DNS records (usually CNAME or A records) that you need to add to your domain registrar.

### Step 3: Configure DNS

Add the DNS records shown in the previous step to your domain provider (like Google Domains, Cloudflare, Namecheap, etc.).

Example DNS configuration:
- **Type**: CNAME
- **Name**: `donate` (or `@` for root domain)
- **Value**: `ghs.googlehosted.com.`

### Step 4: Verify Domain

Domain verification can take a few minutes to a few hours depending on DNS propagation. You can check status with:

```bash
just gcp-domain-list
```

### Managing Domains

```bash
# List all configured domains
just gcp-domain-list

# Get DNS records for a specific domain
just gcp-domain-records donate.yoursite.com

# Delete a domain mapping
just gcp-domain-delete donate.yoursite.com
```

## Common Domain Options

Some domain name ideas for your service:
- `donate.careanimalshelter.org`
- `donorsnap.careanimalshelter.org`
- `donations.careanimalshelter.org`
- `meltingscales-donorsnap-care.run.app` (if you want to use a Cloud Run domain)

## Monitoring and Logs

View real-time logs:

```bash
just gcp-logs
```

## Troubleshooting

### Issue: Domain verification taking too long
- Check that DNS records are correctly configured
- Use `nslookup donate.yoursite.com` to verify DNS propagation
- Wait up to 48 hours for full DNS propagation (usually much faster)

### Issue: Service not accessible
- Verify the service is deployed: `just gcp-url`
- Check logs: `just gcp-logs`
- Ensure the service allows unauthenticated access (default in our justfile)

### Issue: Docker build fails
- Ensure you have the latest Rust toolchain
- Check that all dependencies are available
- Try building locally first: `just docker-build`

## Cost Estimates

Cloud Run pricing (as of 2024):
- **Free tier**: 2 million requests/month, 360,000 GB-seconds/month
- **Beyond free tier**: ~$0.40 per million requests
- **Domain mapping**: Free

For most small organizations, this service will likely stay within the free tier.

## Security Notes

- The service is deployed with `--allow-unauthenticated` by default
- Consider adding authentication if handling sensitive data
- HTTPS is automatically provided by Cloud Run
- Custom domains also get automatic SSL certificates

## Updating the Service

To deploy updates:

```bash
# Make your code changes, then:
just gcp-deploy-all
```

The service will perform a rolling update with zero downtime.
