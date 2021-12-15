use super::*;

/// TODO: Find a better way to traverse paths (i.e. "discardedAt").
/// Maybe try using [`frunk`][frunk]?
///
/// [frunk]: https://github.com/lloydmeta/frunk
#[async_trait]
pub trait Discardable: Entity {
    fn as_discardable(&self) -> DiscardableView;
    fn as_discardable_mut(&mut self) -> DiscardableViewMut;

    fn is_discarded(&self) -> bool {
        let view = self.as_discardable();
        view.discarded_at.is_some()
    }

    fn kept() -> FindQuery<Self> {
        FindQuery::new_untyped(doc! {
            "discardedAt": {
                "$exists": false
            }
        })
    }

    fn discarded() -> FindQuery<Self> {
        FindQuery::new_untyped(doc! {
            "discardedAt": {
                "$exists": true
            }
        })
    }

    async fn discard(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        let view = self.as_discardable_mut();
        *view.discarded_at = Some(now());

        self.before_discard(ctx).await?;
        self.save(ctx).await?;
        self.after_discard(ctx).await?;
        Ok(())
    }

    async fn restore(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        let view = self.as_discardable_mut();
        *view.discarded_at = None;

        self.before_restore(ctx).await?;
        self.save(ctx).await?;
        self.after_restore(ctx).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    async fn before_discard(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn before_restore(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_discard(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_restore(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct DiscardableView<'a> {
    pub discarded_at: &'a Option<DateTime>,
}

#[derive(Debug, Serialize)]
pub struct DiscardableViewMut<'a> {
    pub discarded_at: &'a mut Option<DateTime>,
}
