const LAMPORTS_PER_SOL = 1_000_000_000;

// ========== HELPER PARA CONVERTIR ID A BYTES ==========

function toLeBytes(num: number): Buffer {
  const bn = new anchor.BN(num);
  return bn.toArrayLike(Buffer, "le", 8);
}

// ========== DERIVAR PDAs ==========

function getPlatformPDA() {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("platform")],
    pg.program.programId
  );
}

function getCampaignPDA(campaignId: number) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("campaign"), toLeBytes(campaignId)],
    pg.program.programId
  );
}

function getVaultPDA(campaignId: number) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), toLeBytes(campaignId)],
    pg.program.programId
  );
}

function getDonationPDA(campaignId: number, donor: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("donation"), toLeBytes(campaignId), donor.toBuffer()],
    pg.program.programId
  );
}

// ========== INSTRUCCIONES ==========

async function initializePlatform() {
  const [platformPDA] = getPlatformPDA();

  const tx = await pg.program.methods
    .initialize(new anchor.BN(250))
    .accounts({
      admin: pg.wallet.publicKey,
      platform: platformPDA,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Plataforma inicializada. Tx:", tx);
}

async function createCampaign() {
  const [platformPDA] = getPlatformPDA();
  const platformData = await pg.program.account.platform.fetch(platformPDA);
  const campaignId = platformData.totalCampaigns.toNumber();

  const [campaignPDA] = getCampaignPDA(campaignId);
  const [vaultPDA] = getVaultPDA(campaignId);

  const goal = new anchor.BN(2 * LAMPORTS_PER_SOL);
  const deadline = new anchor.BN(Math.floor(Date.now() / 1000) + 86400);

  const tx = await pg.program.methods
    .createCampaign(
      "Proyecto DeFi Latam",
      "Plataforma de remesas descentralizada",
      goal,
      deadline,
      "https://ejemplo.com/img.png"
    )
    .accounts({
      creator: pg.wallet.publicKey,
      platform: platformPDA,
      campaign: campaignPDA,
      vault: vaultPDA,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Campaña #" + campaignId + " creada. Tx:", tx);
}

async function donateToCampaign(campaignId: number, solAmount: number) {
  const [campaignPDA] = getCampaignPDA(campaignId);
  const [vaultPDA] = getVaultPDA(campaignId);
  const [donationPDA] = getDonationPDA(campaignId, pg.wallet.publicKey);

  const amount = new anchor.BN(solAmount * LAMPORTS_PER_SOL);

  const tx = await pg.program.methods
    .donate(amount)
    .accounts({
      donor: pg.wallet.publicKey,
      campaign: campaignPDA,
      vault: vaultPDA,
      donationRecord: donationPDA,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Donados " + solAmount + " SOL a campaña #" + campaignId + ". Tx:", tx);
}

async function claimFunds(campaignId: number) {
  const [platformPDA] = getPlatformPDA();
  const [campaignPDA] = getCampaignPDA(campaignId);
  const [vaultPDA] = getVaultPDA(campaignId);
  const platformData = await pg.program.account.platform.fetch(platformPDA);

  const tx = await pg.program.methods
    .claimFunds()
    .accounts({
      creator: pg.wallet.publicKey,
      platform: platformPDA,
      campaign: campaignPDA,
      vault: vaultPDA,
      admin: platformData.admin,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Fondos reclamados. Tx:", tx);
}

async function cancelCampaign(campaignId: number) {
  const [campaignPDA] = getCampaignPDA(campaignId);

  const tx = await pg.program.methods
    .cancelCampaign()
    .accounts({
      creator: pg.wallet.publicKey,
      campaign: campaignPDA,
    })
    .rpc();

  console.log("Campaña cancelada. Tx:", tx);
}

async function refundDonation(campaignId: number) {
  const [campaignPDA] = getCampaignPDA(campaignId);
  const [vaultPDA] = getVaultPDA(campaignId);
  const [donationPDA] = getDonationPDA(campaignId, pg.wallet.publicKey);

  const tx = await pg.program.methods
    .refund()
    .accounts({
      donor: pg.wallet.publicKey,
      campaign: campaignPDA,
      vault: vaultPDA,
      donationRecord: donationPDA,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Reembolso procesado. Tx:", tx);
}

// ========== CONSULTAS ==========

async function fetchPlatform() {
  const [platformPDA] = getPlatformPDA();
  const data = await pg.program.account.platform.fetch(platformPDA);
  console.log("--- PLATAFORMA ---");
  console.log("Admin:", data.admin.toBase58());
  console.log("Fee:", data.fee.toNumber(), "bps");
  console.log("Total campañas:", data.totalCampaigns.toNumber());
}

async function fetchCampaign(campaignId: number) {
  const [campaignPDA] = getCampaignPDA(campaignId);
  const d = await pg.program.account.campaign.fetch(campaignPDA);
  const progress = ((d.totalDonated.toNumber() / d.goal.toNumber()) * 100).toFixed(1);
  console.log("--- CAMPAÑA #" + campaignId + " ---");
  console.log("Titulo:", d.title);
  console.log("Meta:", d.goal.toNumber() / LAMPORTS_PER_SOL, "SOL");
  console.log("Recaudado:", d.totalDonated.toNumber() / LAMPORTS_PER_SOL, "SOL (" + progress + "%)");
  console.log("Donantes:", d.donorCount.toNumber());
  console.log("Status:", JSON.stringify(d.status));
  console.log("Reclamado:", d.claimed);
}

async function fetchAllCampaigns() {
  const campaigns = await pg.program.account.campaign.all();
  console.log("--- " + campaigns.length + " CAMPAÑAS ---");
  for (const c of campaigns) {
    const d = c.account;
    console.log(
      "#" + d.campaignId.toNumber() + " " + d.title +
      " | " + d.totalDonated.toNumber() / LAMPORTS_PER_SOL +
      "/" + d.goal.toNumber() / LAMPORTS_PER_SOL + " SOL" +
      " | " + JSON.stringify(d.status)
    );
  }
}

// ========== EJECUTAR ==========

async function main() {
  console.log("Wallet:", pg.wallet.publicKey.toBase58());
  console.log("Program:", pg.program.programId.toBase58());

  const balance = await pg.connection.getBalance(pg.wallet.publicKey);
  console.log("Balance:", balance / LAMPORTS_PER_SOL, "SOL");

  if (balance < 0.5 * LAMPORTS_PER_SOL) {
    console.log("Balance bajo. Ve a https://faucet.solana.com");
    console.log("Pega:", pg.wallet.publicKey.toBase58());
    return;
  }

  // PASO 1 - Solo la primera vez
  await initializePlatform();

  // PASO 2
  await createCampaign();

  // PASO 3
  await donateToCampaign(0, 0.5);

  // PASO 4
  await fetchPlatform();
  await fetchCampaign(0);
  await fetchAllCampaigns();
}

main();
