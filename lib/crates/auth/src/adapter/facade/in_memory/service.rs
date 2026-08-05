use async_trait::async_trait;

use apis::auth::{
    AuthApiError, AuthClaims, AuthService, LoginWithDomainUserInfoRequest,
    LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest, RefreshResponse,
    TokenPair, VerifyRequest,
};
use apis::user::Role as ApiRole;

use crate::domain::{DomainError, DomainIdentityRepository, UserCredentialsRepository};
use crate::usecase::{
    AccessTokenView, AuthClaimsView, AuthUsecase, LoginWithDomainUserInfo, LoginWithPassword,
    Logout as LogoutCmd, RefreshAccessToken, TokenPairView, UsecaseError, VerifyAccessToken,
};

pub struct AuthServiceImpl<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    usecase: AuthUsecase<R, D>,
}

impl<R: UserCredentialsRepository, D: DomainIdentityRepository> AuthServiceImpl<R, D> {
    pub fn new(usecase: AuthUsecase<R, D>) -> Self {
        Self { usecase }
    }
}

fn to_api_role(r: crate::domain::Role) -> ApiRole {
    match r {
        crate::domain::Role::Root => ApiRole::Root,
        crate::domain::Role::Admin => ApiRole::Admin,
        crate::domain::Role::General => ApiRole::General,
    }
}

fn map_error(err: UsecaseError) -> AuthApiError {
    match err {
        UsecaseError::Validation(d) => AuthApiError::Validation(d.to_string()),
        UsecaseError::Repository(d) => match d {
            DomainError::NotFound => AuthApiError::NotFound,
            DomainError::Inactive => AuthApiError::Inactive,
            DomainError::InvalidCredentials => AuthApiError::InvalidCredentials,
            DomainError::Repository(msg) => AuthApiError::Repository(msg),
            DomainError::EmptyCode
            | DomainError::EmptyPasswordHash
            | DomainError::InvalidRole(_)
            | DomainError::DuplicateCode(_) => AuthApiError::Repository(d.to_string()),
        },
        UsecaseError::Verification(msg) => AuthApiError::Verification(msg),
    }
}

#[async_trait]
impl<R: UserCredentialsRepository, D: DomainIdentityRepository> AuthService
    for AuthServiceImpl<R, D>
{
    async fn login_with_password(
        &self,
        req: LoginWithPasswordRequest,
    ) -> Result<TokenPair, AuthApiError> {
        let view: TokenPairView = self
            .usecase
            .login_with_password(LoginWithPassword {
                code: req.code,
                password: req.password,
            })
            .await
            .map_err(map_error)?;
        Ok(TokenPair {
            access_token: view.access_token,
            refresh_token: view.refresh_token,
        })
    }

    async fn login_with_domain_user_info(
        &self,
        req: LoginWithDomainUserInfoRequest,
    ) -> Result<TokenPair, AuthApiError> {
        let view: TokenPairView = self
            .usecase
            .login_with_domain_user_info(LoginWithDomainUserInfo {
                code: req.code,
                domain_name: req.domain_name,
                hostname: req.hostname,
                sid: req.sid,
            })
            .await
            .map_err(map_error)?;
        Ok(TokenPair {
            access_token: view.access_token,
            refresh_token: view.refresh_token,
        })
    }

    async fn logout(&self, req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
        let ack = self
            .usecase
            .logout(LogoutCmd { code: req.code })
            .await
            .map_err(map_error)?;
        Ok(LogoutResponse { code: ack.code })
    }

    async fn verify(&self, req: VerifyRequest) -> Result<AuthClaims, AuthApiError> {
        let view: AuthClaimsView = self
            .usecase
            .verify(VerifyAccessToken {
                access_token: req.access_token,
            })
            .await
            .map_err(map_error)?;
        Ok(AuthClaims {
            code: view.code,
            role: to_api_role(view.role),
            token_version: view.token_version,
        })
    }

    async fn refresh(&self, req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
        let view: AccessTokenView = self
            .usecase
            .refresh(RefreshAccessToken {
                refresh_token: req.refresh_token,
            })
            .await
            .map_err(map_error)?;
        Ok(RefreshResponse {
            access_token: view.access_token,
        })
    }
}
