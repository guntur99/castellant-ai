#!/bin/bash
sed -i '' 's/Booking, Voucher, InvitationTemplate};/Booking, Voucher, InvitationTemplate, Plan, Referral};/' src/handlers.rs

sed -i '' 's/pub all_templates: Vec<InvitationTemplate>,/pub all_templates: Vec<InvitationTemplate>,\n    pub plans: Vec<Plan>,/' src/handlers.rs

# We need more precise replacements. It's better to use sed or write a python script.
