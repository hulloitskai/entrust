use super::*;

#[async_trait]
pub trait Updateable: Entity {
    fn as_updateable(&self) -> UpdateableView;
    fn as_updateable_mut(&mut self) -> UpdateableViewMut;

    async fn update(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        let view = self.as_updateable_mut();
        *view.updated_at = Some(now());

        self.before_update(ctx).await?;
        self.save(ctx).await?;
        self.after_update(ctx).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    async fn before_update(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_update(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct UpdateableView<'a> {
    pub updated_at: &'a Option<DateTime>,
}

#[derive(Debug, Serialize)]
pub struct UpdateableViewMut<'a> {
    pub updated_at: &'a mut Option<DateTime>,
}
