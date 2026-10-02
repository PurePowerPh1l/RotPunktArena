const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const [configPath, bundleDir, version] = process.argv.slice(2);
const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
const publicLines = Buffer.from(config.plugins.updater.pubkey, 'base64').toString('utf8').trim().split(/\r?\n/);
const publicBytes = Buffer.from(publicLines[1], 'base64');
assert.equal(publicBytes.length, 42);
const key = crypto.createPublicKey({key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), publicBytes.subarray(10)]), format:'der', type:'spki'});
for (const file of [`RotPunktArena_${version}_x64-setup.exe`, `RotPunktArena_${version}_x64_en-US.msi`]) {
  const payload = fs.readFileSync(path.join(bundleDir, file));
  const lines = Buffer.from(fs.readFileSync(path.join(bundleDir, file+'.sig'), 'utf8').trim(), 'base64').toString('utf8').trim().split(/\r?\n/);
  assert.equal(lines.length, 4);
  const signature = Buffer.from(lines[1], 'base64');
  assert.equal(signature.length, 74);
  assert.deepEqual(signature.subarray(2,10), publicBytes.subarray(2,10));
  assert.ok(['Ed','ED'].includes(signature.subarray(0,2).toString('ascii')));
  const signData = data => signature.subarray(0,2).toString('ascii') === 'ED' ? crypto.createHash('blake2b512').update(data).digest() : data;
  assert.ok(crypto.verify(null, signData(payload), key, signature.subarray(10)), 'artifact signature');
  assert.ok(lines[2].startsWith('trusted comment: '));
  assert.ok(crypto.verify(null, Buffer.concat([signature.subarray(10), Buffer.from(lines[2].slice(17))]), key, Buffer.from(lines[3], 'base64')), 'trusted comment signature');
  console.log(JSON.stringify({file,size:payload.length,sha256:crypto.createHash('sha256').update(payload).digest('hex'),signature:'valid'}));
}
const manifest = JSON.parse(fs.readFileSync(path.join(bundleDir, 'latest.json'), 'utf8'));
assert.equal(manifest.version, version);
assert.equal(manifest.platforms['windows-x86_64'].signature, fs.readFileSync(path.join(bundleDir, `RotPunktArena_${version}_x64-setup.exe.sig`), 'utf8').trim());
assert.equal(manifest.platforms['windows-x86_64'].url, `https://github.com/PurePowerPh1l/RotPunktArena/releases/download/v${version}/RotPunktArena_${version}_x64-setup.exe`);
console.log('Updater manifest and production signatures verified');
