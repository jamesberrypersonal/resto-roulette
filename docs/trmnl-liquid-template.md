# TRMNL Liquid Template

This file is the source of truth to paste into the TRMNL Private Plugin UI. The template itself is hosted on TRMNL's servers — it is not stored in this repo beyond this reference.

## JSON contract

`GET /trmnl?token=<RESTO_AUTH_TOKEN>` returns:

```json
{
  "generated_at": "2026-05-02T08:00:00Z",
  "near": {
    "name": "Hà",
    "address": "243 Rue De Bleury",
    "duration_minutes": 12,
    "mode": "walk",
    "cuisine": "vietnamese"
  },
  "mid": {
    "name": "Schwartz's",
    "address": "3895 Saint-Laurent Blvd",
    "duration_minutes": 22,
    "mode": "bike",
    "cuisine": null
  },
  "far": {
    "name": "Joe Beef",
    "address": "2491 Notre-Dame St W",
    "duration_minutes": 38,
    "mode": "drive",
    "cuisine": null
  }
}
```

- `near` / `mid` / `far` are `null` when the corresponding bucket is empty.
- `cuisine` is `null` when not in the Places API cache.
- `mode` is one of `walk` / `bike` / `transit` / `drive`.
- `duration_minutes` is the best travel time across eligible modes for that bucket.

## Liquid template

```liquid
<div class="layout">

  <div class="bucket">
    <span class="label">Near · walk / bike / transit ≤15 min</span>
    {% if near %}
      <span class="name">{{ near.name }}{% if near.cuisine %} · {{ near.cuisine | capitalize }}{% endif %}</span>
      <span class="detail">~{{ near.duration_minutes }} min by {{ near.mode }}</span>
    {% else %}
      <span class="empty">—</span>
    {% endif %}
  </div>

  <div class="bucket">
    <span class="label">Mid · bike / transit 15–30 min</span>
    {% if mid %}
      <span class="name">{{ mid.name }}{% if mid.cuisine %} · {{ mid.cuisine | capitalize }}{% endif %}</span>
      <span class="detail">~{{ mid.duration_minutes }} min by {{ mid.mode }}</span>
    {% else %}
      <span class="empty">—</span>
    {% endif %}
  </div>

  <div class="bucket">
    <span class="label">Far · bike / transit / drive 30–60 min</span>
    {% if far %}
      <span class="name">{{ far.name }}{% if far.cuisine %} · {{ far.cuisine | capitalize }}{% endif %}</span>
      <span class="detail">~{{ far.duration_minutes }} min by {{ far.mode }}</span>
    {% else %}
      <span class="empty">—</span>
    {% endif %}
  </div>

  <div class="footer">{{ generated_at | date: "%b %-d" }}</div>

</div>
```

Adjust class names and layout markup to match the TRMNL framework's CSS conventions when pasting into the plugin UI.
