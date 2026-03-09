use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("11111111111111111111111111111111");

#[program]
pub mod crowdfund {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, platform_fee: u64) -> Result<()> {
        let platform = &mut ctx.accounts.platform;
        platform.admin = ctx.accounts.admin.key();
        platform.fee = platform_fee;
        platform.total_campaigns = 0;
        platform.bump = ctx.bumps.platform;
        msg!("Plataforma inicializada con fee: {}bps", platform_fee);
        Ok(())
    }

    pub fn create_campaign(
        ctx: Context<CreateCampaign>,
        title: String,
        description: String,
        goal: u64,
        deadline: i64,
        image_url: String,
    ) -> Result<()> {
        require!(title.len() <= 50, CrowdfundError::TitleTooLong);
        require!(description.len() <= 200, CrowdfundError::DescriptionTooLong);
        require!(image_url.len() <= 200, CrowdfundError::UrlTooLong);
        require!(goal > 0, CrowdfundError::InvalidGoal);

        let clock = Clock::get()?;
        require!(deadline > clock.unix_timestamp, CrowdfundError::InvalidDeadline);

        let platform = &mut ctx.accounts.platform;
        let campaign = &mut ctx.accounts.campaign;

        campaign.creator = ctx.accounts.creator.key();
        campaign.title = title.clone();
        campaign.description = description;
        campaign.image_url = image_url;
        campaign.goal = goal;
        campaign.total_donated = 0;
        campaign.donor_count = 0;
        campaign.deadline = deadline;
        campaign.claimed = false;
        campaign.status = CampaignStatus::Active;
        campaign.created_at = clock.unix_timestamp;
        campaign.campaign_id = platform.total_campaigns;
        campaign.bump = ctx.bumps.campaign;
        campaign.vault_bump = ctx.bumps.vault;

        platform.total_campaigns += 1;

        msg!("Campaña #{} creada: {}", campaign.campaign_id, title);
        Ok(())
    }

    pub fn donate(ctx: Context<Donate>, amount: u64) -> Result<()> {
        require!(amount > 0, CrowdfundError::InvalidAmount);

        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        require!(
            campaign.status == CampaignStatus::Active,
            CrowdfundError::CampaignNotActive
        );
        require!(
            clock.unix_timestamp <= campaign.deadline,
            CrowdfundError::CampaignExpired
        );

        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.donor.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, amount)?;

        let donation = &mut ctx.accounts.donation_record;
        donation.donor = ctx.accounts.donor.key();
        donation.campaign_id = campaign.campaign_id;
        donation.amount = donation.amount.checked_add(amount).unwrap();
        donation.timestamp = clock.unix_timestamp;
        donation.bump = ctx.bumps.donation_record;

        campaign.total_donated = campaign.total_donated.checked_add(amount).unwrap();
        campaign.donor_count += 1;

        if campaign.total_donated >= campaign.goal {
            campaign.status = CampaignStatus::Funded;
            msg!("¡Campaña #{} alcanzó su meta!", campaign.campaign_id);
        }

        msg!("Donación de {} lamports a campaña #{}", amount, campaign.campaign_id);
        Ok(())
    }

    pub fn claim_funds(ctx: Context<ClaimFunds>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        require!(
            campaign.status == CampaignStatus::Funded
                || (campaign.status == CampaignStatus::Active
                    && clock.unix_timestamp > campaign.deadline
                    && campaign.total_donated > 0),
            CrowdfundError::CannotClaim
        );
        require!(!campaign.claimed, CrowdfundError::AlreadyClaimed);

        let platform = &ctx.accounts.platform;
        let vault = &ctx.accounts.vault;

        let total = vault.lamports();
        let rent_exempt = Rent::get()?.minimum_balance(0);
        let available = total.saturating_sub(rent_exempt);

        let fee_amount = available
            .checked_mul(platform.fee)
            .unwrap()
            .checked_div(10_000)
            .unwrap();
        let creator_amount = available.checked_sub(fee_amount).unwrap();

        **vault.to_account_info().try_borrow_mut_lamports()? -= creator_amount;
        **ctx.accounts.creator.to_account_info().try_borrow_mut_lamports()? += creator_amount;

        if fee_amount > 0 {
            **vault.to_account_info().try_borrow_mut_lamports()? -= fee_amount;
            **ctx.accounts.admin.to_account_info().try_borrow_mut_lamports()? += fee_amount;
        }

        campaign.claimed = true;
        campaign.status = CampaignStatus::Claimed;

        msg!("Fondos reclamados: {} lamports (fee: {})", creator_amount, fee_amount);
        Ok(())
    }

    pub fn cancel_campaign(ctx: Context<CancelCampaign>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;

        require!(
            campaign.status == CampaignStatus::Active,
            CrowdfundError::CampaignNotActive
        );
        require!(campaign.total_donated == 0, CrowdfundError::CampaignHasDonations);

        campaign.status = CampaignStatus::Cancelled;
        msg!("Campaña #{} cancelada", campaign.campaign_id);
        Ok(())
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        let campaign = &ctx.accounts.campaign;
        let clock = Clock::get()?;

        require!(clock.unix_timestamp > campaign.deadline, CrowdfundError::CampaignNotExpired);
        require!(campaign.total_donated < campaign.goal, CrowdfundError::GoalReached);
        require!(!campaign.claimed, CrowdfundError::AlreadyClaimed);

        let donation = &mut ctx.accounts.donation_record;
        require!(donation.amount > 0, CrowdfundError::NoDonation);

        let refund_amount = donation.amount;

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= refund_amount;
        **ctx.accounts.donor.to_account_info().try_borrow_mut_lamports()? += refund_amount;

        donation.amount = 0;
        msg!("Reembolso de {} lamports", refund_amount);
        Ok(())
    }
}

// ===================== CUENTAS =====================

#[account]
#[derive(InitSpace)]
pub struct Platform {
    pub admin: Pubkey,
    pub fee: u64,
    pub total_campaigns: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Campaign {
    pub creator: Pubkey,
    #[max_len(50)]
    pub title: String,
    #[max_len(200)]
    pub description: String,
    #[max_len(200)]
    pub image_url: String,
    pub goal: u64,
    pub total_donated: u64,
    pub donor_count: u64,
    pub deadline: i64,
    pub claimed: bool,
    pub status: CampaignStatus,
    pub created_at: i64,
    pub campaign_id: u64,
    pub bump: u8,
    pub vault_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct DonationRecord {
    pub donor: Pubkey,
    pub campaign_id: u64,
    pub amount: u64,
    pub timestamp: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum CampaignStatus {
    Active,
    Funded,
    Claimed,
    Cancelled,
}

// ===================== CONTEXTOS =====================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + Platform::INIT_SPACE,
        seeds = [b"platform"],
        bump
    )]
    pub platform: Account<'info, Platform>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(title: String, description: String, goal: u64, deadline: i64, image_url: String)]
pub struct CreateCampaign<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        mut,
        seeds = [b"platform"],
        bump = platform.bump
    )]
    pub platform: Account<'info, Platform>,
    #[account(
        init,
        payer = creator,
        space = 8 + Campaign::INIT_SPACE,
        seeds = [b"campaign", platform.total_campaigns.to_le_bytes().as_ref()],
        bump
    )]
    pub campaign: Account<'info, Campaign>,
    /// CHECK: Vault PDA para almacenar fondos
    #[account(
        init,
        payer = creator,
        space = 0,
        seeds = [b"vault", platform.total_campaigns.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Donate<'info> {
    #[account(mut)]
    pub donor: Signer<'info>,
    #[account(
        mut,
        seeds = [b"campaign", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, Campaign>,
    /// CHECK: Vault PDA
    #[account(
        mut,
        seeds = [b"vault", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.vault_bump
    )]
    pub vault: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = donor,
        space = 8 + DonationRecord::INIT_SPACE,
        seeds = [b"donation", campaign.campaign_id.to_le_bytes().as_ref(), donor.key().as_ref()],
        bump
    )]
    pub donation_record: Account<'info, DonationRecord>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimFunds<'info> {
    #[account(
        mut,
        constraint = creator.key() == campaign.creator @ CrowdfundError::Unauthorized
    )]
    pub creator: Signer<'info>,
    #[account(seeds = [b"platform"], bump = platform.bump)]
    pub platform: Account<'info, Platform>,
    #[account(
        mut,
        seeds = [b"campaign", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, Campaign>,
    /// CHECK: Vault PDA
    #[account(
        mut,
        seeds = [b"vault", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.vault_bump
    )]
    pub vault: AccountInfo<'info>,
    /// CHECK: Admin wallet
    #[account(
        mut,
        constraint = admin.key() == platform.admin @ CrowdfundError::Unauthorized
    )]
    pub admin: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelCampaign<'info> {
    #[account(
        mut,
        constraint = creator.key() == campaign.creator @ CrowdfundError::Unauthorized
    )]
    pub creator: Signer<'info>,
    #[account(
        mut,
        seeds = [b"campaign", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, Campaign>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub donor: Signer<'info>,
    #[account(
        seeds = [b"campaign", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, Campaign>,
    /// CHECK: Vault PDA
    #[account(
        mut,
        seeds = [b"vault", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.vault_bump
    )]
    pub vault: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"donation", campaign.campaign_id.to_le_bytes().as_ref(), donor.key().as_ref()],
        bump = donation_record.bump,
        constraint = donation_record.donor == donor.key() @ CrowdfundError::Unauthorized
    )]
    pub donation_record: Account<'info, DonationRecord>,
    pub system_program: Program<'info, System>,
}

// ===================== ERRORES =====================

#[error_code]
pub enum CrowdfundError {
    #[msg("Titulo muy largo")]
    TitleTooLong,
    #[msg("Descripcion muy larga")]
    DescriptionTooLong,
    #[msg("URL muy larga")]
    UrlTooLong,
    #[msg("Meta invalida")]
    InvalidGoal,
    #[msg("Deadline invalido")]
    InvalidDeadline,
    #[msg("Campaña no activa")]
    CampaignNotActive,
    #[msg("Campaña expirada")]
    CampaignExpired,
    #[msg("Monto invalido")]
    InvalidAmount,
    #[msg("No se puede reclamar")]
    CannotClaim,
    #[msg("Ya reclamado")]
    AlreadyClaimed,
    #[msg("No autorizado")]
    Unauthorized,
    #[msg("Campaña tiene donaciones")]
    CampaignHasDonations,
    #[msg("Campaña no expirada")]
    CampaignNotExpired,
    #[msg("Meta alcanzada")]
    GoalReached,
    #[msg("Sin donacion")]
    NoDonation,
}
