#!/usr/bin/env python3
import re

# Read the test file
with open('contract/contracts/predifi-contract/src/test.rs', 'r') as f:
    content = f.read()

# 1. Update setup() to add creator
# Add creator variable after operator
content = content.replace(
    '    let operator = Address::generate(env);\n\n    ac_client.grant_role',
    '    let operator = Address::generate(env);\n    let creator = Address::generate(env);\n\n    ac_client.grant_role'
)

# 2. Update setup return type to include creator (8 elements)
old_return_type = ''') -> (
    dummy_access_control::DummyAccessControlClient<'_>,
    PredifiContractClient<'_>,
    Address,
    token::Client<'_>,
    token::StellarAssetClient<'_>,
    Address,
    Address,
) {'''

new_return_type = ''') -> (
    dummy_access_control::DummyAccessControlClient<'_>,
    PredifiContractClient<'_>,
    Address,
    token::Client<'_>,
    token::StellarAssetClient<'_>,
    Address,
    Address,
    Address,
) {'''

content = content.replace(old_return_type, new_return_type)

# 3. Update setup return to include creator
old_return = '''    (
        ac_client,
        client,
        token_address,
        token,
        token_admin_client,
        treasury,
        operator,
    )'''

new_return = '''    (
        ac_client,
        client,
        token_address,
        token,
        token_admin_client,
        treasury,
        operator,
        creator,
    )'''

content = content.replace(old_return, new_return)

# 4. Update all setup() destructuring patterns to include creator
destruct_patterns = [
    ('let (_, client, token_address, token, token_admin_client, _, operator) = setup',
     'let (_, client, token_address, token, token_admin_client, _, operator, creator) = setup'),
    ('let (_, client, token_address, _, token_admin_client, _, operator) = setup',
     'let (_, client, token_address, _, token_admin_client, _, operator, creator) = setup'),
    ('let (_, client, token_address, _, token_admin_client, _, _) = setup',
     'let (_, client, token_address, _, token_admin_client, _, _, creator) = setup'),
    ('let (_, client, token_address, _, _, _, _) = setup',
     'let (_, client, token_address, _, _, _, creator) = setup'),
    ('let (_, client, _, _, _, _, _) = setup',
     'let (_, client, _, _, _, _, creator) = setup'),
]

for old_pat, new_pat in destruct_patterns:
    content = content.replace(old_pat + '(&env);', new_pat + '(&env);')

# 5. Add creator and initial_liquidity to create_pool calls
# For each create_pool call, we need to add &creator as first arg and &0i128 as last arg

# Find patterns and fix them
# Pattern 1: create_pool(\n        &100000u64,
def add_creator_and_liquidity(match):
    block = match.group(0)
    # Check if already fixed
    if '&creator,' in block:
        return block
    
    # Add creator after opening paren
    block = block.replace('(\n        &100000u64,', '(\n        &creator,\n        &100000u64,')
    
    # Add initial_liquidity at the end before closing
    # Find patterns like: "ipfs://..."),\n    );
    if '&0i128' not in block:
        block = block.replace(
            '            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",\n        ),',
            '            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",\n        ),\n        &0i128,'
        )
        block = block.replace(
            '            "ipfs://metadata",\n        ),',
            '            "ipfs://metadata",\n        ),\n        &0i128,'
        )
    
    return block

# Find all create_pool blocks and fix them
# Match from "let pool_id = client.create_pool(" to the closing ");"
pattern = r'(let pool_id = client\.create_pool\([^;]+?\);)'
content = re.sub(pattern, add_creator_and_liquidity, content, flags=re.DOTALL)

# Also fix create_pool calls without pool_id assignment
pattern2 = r'(client\.create_pool\([^;]+?\);)'
def add_creator_and_liquidity2(match):
    block = match.group(0)
    if '&creator,' in block:
        return block
    # Check if this is inside a should_panic test (no pool_id)
    if 'let pool_id' not in block:
        # Add creator after (
        if '&100000u64,' in block:
            block = block.replace('(\n        &100000u64,', '(\n        &creator,\n        &100000u64,')
        # Add initial_liquidity
        if '&0i128' not in block:
            block = block.replace(
                '            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",\n        ),',
                '            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",\n        ),\n        &0i128,'
            )
            block = block.replace(
                '            "ipfs://metadata",\n        ),',
                '            "ipfs://metadata",\n        ),\n        &0i128,'
            )
    return block

content = re.sub(pattern2, add_creator_and_liquidity2, content, flags=re.DOTALL)

# Write back
with open('contract/contracts/predifi-contract/src/test.rs', 'w') as f:
    f.write(content)

print("Done")
